use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use pyo3::prelude::*;
use tokio::sync::mpsc;
use topk_rs::proto::v1::ctx::file::InputFile;

use crate::client::sync::runtime::Runtime;
use crate::client::NativeWaitConfig;
use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::file::FileOrFileLike;
use crate::data::list_entry::ListEntry;
use crate::data::value::Value;
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

#[pyclass]
pub struct DatasetClient {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    dataset: String,
}

impl DatasetClient {
    pub fn new(runtime: Arc<Runtime>, client: Arc<topk_rs::Client>, dataset: String) -> Self {
        Self {
            runtime,
            client,
            dataset,
        }
    }
}

#[pymethods]
impl DatasetClient {
    #[pyo3(signature = (doc_id, input, metadata))]
    pub fn upsert_file(
        &self,
        py: Python<'_>,
        doc_id: String,
        input: FileOrFileLike,
        metadata: HashMap<String, Value>,
    ) -> PyResult<String> {
        let input_file: InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let handle = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .upsert_file(doc_id, input_file, metadata),
            )
            .map_err(RustError)?;

        Ok(handle)
    }

    #[pyo3(signature = (ids, fields=None))]
    pub fn get_metadata(
        &self,
        py: Python<'_>,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
    ) -> PyResult<HashMap<String, HashMap<String, Value>>> {
        let docs = self
            .runtime
            .block_on(
                py,
                self.client.dataset(&self.dataset).get_metadata(ids, fields),
            )
            .map_err(RustError)?;

        let docs = docs
            .into_iter()
            .map(|(id, doc)| {
                (
                    id,
                    doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
                )
            })
            .collect();

        Ok(docs)
    }

    pub fn update_metadata(
        &self,
        py: Python<'_>,
        doc_id: String,
        metadata: HashMap<String, Value>,
    ) -> PyResult<String> {
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let handle = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .update_metadata(doc_id, metadata),
            )
            .map_err(RustError)?;

        Ok(handle)
    }

    pub fn delete(&self, py: Python<'_>, doc_id: String) -> PyResult<String> {
        let handle = self
            .runtime
            .block_on(py, self.client.dataset(&self.dataset).delete(doc_id))
            .map_err(RustError)?;

        Ok(handle)
    }

    pub fn check_handle(&self, py: Python<'_>, handle: String) -> PyResult<bool> {
        Ok(self
            .runtime
            .block_on(py, self.client.dataset(&self.dataset).check_handle(&handle))
            .map_err(RustError)?)
    }

    #[pyo3(signature = (handle, config=None))]
    pub fn wait_for_handle(
        &self,
        py: Python<'_>,
        handle: String,
        config: Option<NativeWaitConfig>,
    ) -> PyResult<()> {
        let wait_config = config.map(|c| c.config.into());
        self.runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .wait_for_handle(&handle, wait_config),
            )
            .map_err(RustError)?;
        Ok(())
    }

    #[pyo3(signature = (fields=None, filter=None))]
    pub fn list(
        &self,
        fields: Option<Vec<String>>,
        filter: Option<LogicalExpr>,
    ) -> PyResult<DatasetListIterator> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let filter = filter.map(|f| f.into());

        self.runtime.spawn({
            let client = self.client.clone();
            let dataset = self.dataset.clone();
            async move {
                let mut stream = match client.dataset(&dataset).list(fields, filter).await {
                    Ok(stream) => stream,
                    Err(e) => {
                        let _ = tx.send(Err(RustError(e).into())).await;
                        return;
                    }
                };

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(entry) => {
                            if let Err(mpsc::error::SendError(_)) = tx.send(Ok(entry.into())).await
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

        Ok(DatasetListIterator {
            runtime: self.runtime.clone(),
            receiver: rx,
        })
    }
}

#[pyclass]
pub struct DatasetListIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<ListEntry>>,
}

#[pymethods]
impl DatasetListIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<ListEntry>> {
        self.runtime
            .block_on(py, async { self.receiver.recv().await.transpose() })
    }
}
