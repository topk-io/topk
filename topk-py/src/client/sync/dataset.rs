use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use pyo3::prelude::*;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::client::sync::runtime::Runtime;
use crate::client::LIST_ENTRIES_BUFFER_SIZE;
use crate::client::{
    into_py_response, CheckHandleResponse, DeleteFileResponse, GetMetadataResponse, Response,
    UpdateMetadataResponse, UpsertFileResponse,
};
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
    #[pyo3(signature = (file_id, input, metadata))]
    pub fn upsert_file(
        &self,
        py: Python<'_>,
        file_id: String,
        input: FileOrFileLike,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<UpsertFileResponse>> {
        let input_file: topk_rs::proto::v1::ctx::file::InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .upsert_file(file_id, input_file, metadata),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(UpsertFileResponse {
                handle: inner.handle,
            })
        })
    }

    #[pyo3(signature = (file_id, fields=None))]
    pub fn get_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
        fields: Option<Vec<String>>,
    ) -> PyResult<Py<GetMetadataResponse>> {
        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .get_metadata(vec![file_id], fields),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            let metadata: HashMap<String, Value> = inner
                .docs
                .into_iter()
                .flat_map(|(_, doc)| doc.fields.into_iter())
                .map(|(k, v)| (k, v.into()))
                .collect();
            Ok(GetMetadataResponse { metadata })
        })
    }

    pub fn update_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<UpdateMetadataResponse>> {
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .update_metadata(file_id, metadata),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(UpdateMetadataResponse {
                handle: inner.handle,
            })
        })
    }

    pub fn delete(&self, py: Python<'_>, file_id: String) -> PyResult<Py<DeleteFileResponse>> {
        let response = self
            .runtime
            .block_on(py, self.client.dataset(&self.dataset).delete(file_id))
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(DeleteFileResponse {
                handle: inner.handle,
            })
        })
    }

    pub fn check_handle(
        &self,
        py: Python<'_>,
        handle: String,
    ) -> PyResult<Py<CheckHandleResponse>> {
        let processed = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .check_handle(&handle),
            )
            .map_err(RustError)?;

        let init = pyo3::PyClassInitializer::from(Response { request_id: None })
            .add_subclass(CheckHandleResponse { processed });
        Py::new(py, init)
    }

    #[pyo3(signature = (fields=None, filter=None))]
    pub fn list(
        &self,
        fields: Option<Vec<String>>,
        filter: Option<LogicalExpr>,
    ) -> PyResult<DatasetListIterator> {
        let (tx, rx) = mpsc::channel(LIST_ENTRIES_BUFFER_SIZE);
        let filter = filter.map(|f| f.into());

        let handle = self.runtime.spawn({
            let client = self.client.clone();
            let dataset = self.dataset.clone();
            async move {
                let mut stream = match client.dataset(&dataset).list(fields, filter).await {
                    Ok(response) => response.into_inner(),
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
            handle,
        })
    }
}

#[pyclass]
pub struct DatasetListIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<ListEntry>>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
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

impl Drop for DatasetListIterator {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
