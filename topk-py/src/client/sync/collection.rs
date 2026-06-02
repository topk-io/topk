use crate::client::sync::runtime::Runtime;
use crate::client::Document;
use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::partition::Partition;
use crate::data::value::Value;
use crate::error::RustError;
use crate::expr::delete::DeleteExprUnion;
use crate::query::{ConsistencyLevel, Query};
use futures_util::StreamExt;
use pyo3::prelude::*;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;

#[pyclass]
pub struct CollectionClient {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    collection: String,
    partition: Option<String>,
}

impl CollectionClient {
    pub fn new(
        runtime: Arc<Runtime>,
        client: Arc<topk_rs::Client>,
        collection: String,
        partition: Option<String>,
    ) -> Self {
        Self {
            runtime,
            client,
            collection,
            partition,
        }
    }

    /// Get partition-aware collection client
    fn collection(&self) -> topk_rs::CollectionClient {
        let c = self.client.collection(&self.collection);
        match &self.partition {
            Some(p) => c.partition(p.clone()),
            None => c,
        }
    }
}

#[pymethods]
impl CollectionClient {
    #[pyo3(signature = (ids, fields=None, lsn=None, consistency=None))]
    pub fn get(
        &self,
        py: Python<'_>,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<HashMap<String, Document>> {
        let docs = self
            .runtime
            .block_on(
                py,
                self.collection().get(
                    ids,
                    fields,
                    lsn,
                    consistency.map(|c| c.into()),
                ),
            )
            .map_err(RustError)?;

        Ok(docs
            .into_iter()
            .map(|(id, doc)| (id, Document::from(doc)))
            .collect())
    }

    #[pyo3(signature = (lsn=None, consistency=None))]
    pub fn count(
        &self,
        py: Python<'_>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<u64> {
        let count = self
            .runtime
            .block_on(
                py,
                self.collection()
                    .count(lsn, consistency.map(|c| c.into())),
            )
            .map_err(RustError)?;

        Ok(count)
    }

    #[pyo3(signature = (query, lsn=None, consistency=None))]
    pub fn query(
        &self,
        py: Python<'_>,
        query: Query,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<Vec<Document>> {
        // Convert query to proto while GIL is held
        let query: topk_rs::proto::v1::data::Query = query.into();

        let docs = self
            .runtime
            .block_on(
                py,
                self.collection()
                    .query(query, lsn, consistency.map(|c| c.into())),
            )
            .map_err(RustError)?;

        let docs: Vec<Document> = docs.into_iter().map(|d| d.into()).collect();

        Ok(docs)
    }

    pub fn upsert(
        &self,
        py: Python<'_>,
        documents: Vec<HashMap<String, Value>>,
    ) -> PyResult<String> {
        let documents = documents
            .into_iter()
            .map(|d| topk_rs::proto::v1::data::Document {
                fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })
            .collect();

        Ok(self
            .runtime
            .block_on(py, self.collection().upsert(documents))
            .map_err(RustError)?)
    }

    pub fn update(
        &self,
        py: Python<'_>,
        documents: Vec<HashMap<String, Value>>,
        fail_on_missing: Option<bool>,
    ) -> PyResult<String> {
        let documents = documents
            .into_iter()
            .map(|d| topk_rs::proto::v1::data::Document {
                fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })
            .collect();

        Ok(self
            .runtime
            .block_on(
                py,
                self.collection()
                    .update(documents, fail_on_missing.unwrap_or(false)),
            )
            .map_err(RustError)?)
    }

    pub fn delete(&self, py: Python<'_>, spec: DeleteExprUnion) -> PyResult<String> {
        // Convert spec to proto while GIL is held
        let spec: topk_rs::proto::v1::data::DeleteDocumentsRequest = spec.into();

        Ok(self
            .runtime
            .block_on(py, self.collection().delete(spec))
            .map_err(RustError)?)
    }

    #[pyo3(signature = (prefix=None))]
    pub fn list_partitions(&self, prefix: Option<String>) -> PyResult<PartitionListIterator> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        self.runtime.spawn({
            let client = self.client.clone();
            let collection = self.collection.clone();
            async move {
                let mut stream = match client.collection(&collection).list_partitions(prefix).await
                {
                    Ok(stream) => stream,
                    Err(e) => {
                        let _ = tx.send(Err(RustError(e).into())).await;
                        return;
                    }
                };

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(partition) => {
                            if let Err(mpsc::error::SendError(_)) =
                                tx.send(Ok(partition.into())).await
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(RustError(e.into()).into())).await;
                            break;
                        }
                    }
                }
            }
        });

        Ok(PartitionListIterator {
            runtime: self.runtime.clone(),
            receiver: rx,
        })
    }

    pub fn delete_partition(&self, py: Python<'_>, name: String) -> PyResult<()> {
        self.runtime
            .block_on(
                py,
                self.client
                    .collection(&self.collection)
                    .delete_partition(name),
            )
            .map_err(RustError)?;
        Ok(())
    }
}

#[pyclass]
pub struct PartitionListIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<Partition>>,
}

#[pymethods]
impl PartitionListIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Partition>> {
        self.runtime
            .block_on(py, async { self.receiver.recv().await.transpose() })
    }
}
