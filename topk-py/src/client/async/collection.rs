use crate::client::Document;
use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::partition::Partition;
use crate::data::value::Value;
use crate::error::RustError;
use crate::expr::delete::DeleteExprUnion;
use crate::query::{ConsistencyLevel, Query};
use futures_util::StreamExt;
use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex};

#[pyclass]
pub struct AsyncCollectionClient {
    client: Arc<topk_rs::Client>,
    collection: Arc<String>,
    partition: Option<String>,
}

impl AsyncCollectionClient {
    pub fn new(
        client: Arc<topk_rs::Client>,
        collection: Arc<String>,
        partition: Option<String>,
    ) -> Self {
        Self {
            client,
            collection,
            partition,
        }
    }

    /// Get partition-aware collection client
    fn collection(&self) -> topk_rs::CollectionClient {
        let c = self.client.collection(self.collection.as_str());
        match &self.partition {
            Some(p) => c.partition(p.clone()),
            None => c,
        }
    }
}

#[pymethods]
impl AsyncCollectionClient {
    #[pyo3(signature = (ids, fields=None, lsn=None, consistency=None))]
    pub fn get(
        &self,
        py: Python<'_>,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<Py<PyAny>> {
        let collection = self.collection();

        future_into_py(py, async move {
            let docs = collection
                .get(ids, fields, lsn, consistency.map(|c| c.into()))
                .await
                .map_err(RustError)?;

            let docs: HashMap<String, Document> = docs
                .into_iter()
                .map(|(id, doc)| (id, Document::from(doc)))
                .collect();

            Ok(docs)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (lsn=None, consistency=None))]
    pub fn count(
        &self,
        py: Python<'_>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<Py<PyAny>> {
        let collection = self.collection();

        future_into_py(py, async move {
            let count = collection
                .count(lsn, consistency.map(|c| c.into()))
                .await
                .map_err(RustError)?;

            Ok(count)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (query, lsn=None, consistency=None))]
    pub fn query(
        &self,
        py: Python<'_>,
        query: Query,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<Py<PyAny>> {
        // Convert query to proto while GIL is held
        let query = query.into();
        let collection = self.collection();

        future_into_py(py, async move {
            let docs = collection
                .query(query, lsn, consistency.map(|c| c.into()))
                .await
                .map_err(RustError)?;

            let docs: Vec<Document> = docs.into_iter().map(|d| d.into()).collect();

            Ok(docs)
        })
        .map(|result| result.into())
    }

    pub fn upsert(
        &self,
        py: Python<'_>,
        documents: Vec<HashMap<String, Value>>,
    ) -> PyResult<Py<PyAny>> {
        let collection = self.collection();

        future_into_py(py, async move {
            let documents = documents
                .into_iter()
                .map(|d| topk_rs::proto::v1::data::Document {
                    fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
                })
                .collect();

            let lsn = collection.upsert(documents).await.map_err(RustError)?;

            Ok(lsn)
        })
        .map(|result| result.into())
    }

    pub fn update(
        &self,
        py: Python<'_>,
        documents: Vec<HashMap<String, Value>>,
        fail_on_missing: Option<bool>,
    ) -> PyResult<Py<PyAny>> {
        let documents = documents
            .into_iter()
            .map(|d| topk_rs::proto::v1::data::Document {
                fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })
            .collect();
        let collection = self.collection();

        future_into_py(py, async move {
            let lsn = collection
                .update(documents, fail_on_missing.unwrap_or(false))
                .await
                .map_err(RustError)?;

            Ok(lsn)
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, spec: DeleteExprUnion) -> PyResult<Py<PyAny>> {
        // Convert spec to proto while GIL is held
        let spec: topk_rs::proto::v1::data::DeleteDocumentsRequest = spec.into();
        let collection = self.collection();

        future_into_py(py, async move {
            let lsn = collection.delete(spec).await.map_err(RustError)?;

            Ok(lsn)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (prefix=None))]
    pub fn list_partitions(
        &self,
        _py: Python<'_>,
        prefix: Option<String>,
    ) -> PyResult<AsyncPartitionListIterator> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let client = self.client.clone();
        let collection = self.collection.clone();

        pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
            let mut stream = match client
                .collection(collection.as_str())
                .list_partitions(prefix)
                .await
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
                        if let Err(mpsc::error::SendError(_)) = tx.send(Ok(partition.into())).await
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
        });

        Ok(AsyncPartitionListIterator {
            receiver: Arc::new(Mutex::new(rx)),
        })
    }

    pub fn delete_partition(&self, py: Python<'_>, name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        future_into_py(py, async move {
            client
                .collection(collection.as_str())
                .delete_partition(name)
                .await
                .map_err(RustError)?;
            Ok(())
        })
        .map(|result| result.into())
    }
}

#[pyclass]
pub struct AsyncPartitionListIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<Partition>>>>,
}

#[pymethods]
impl AsyncPartitionListIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRefMut<'_, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let receiver = slf.receiver.clone();
        future_into_py(py, async move {
            let mut receiver = receiver.lock().await;
            match receiver.recv().await.transpose() {
                Ok(Some(partition)) => Ok(partition),
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Stream exhausted",
                )),
                Err(e) => Err(e),
            }
        })
    }
}
