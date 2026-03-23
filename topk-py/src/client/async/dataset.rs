use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, Mutex};
use topk_rs::proto::v1::ctx::file::InputFile;

use crate::client::into_py_response;
use crate::client::CHANNEL_BUFFER_SIZE;
use crate::client::{
    DeleteFileResponse, GetMetadataResponse, NativeWaitConfig, UpdateMetadataResponse,
    UpsertResponse,
};
use crate::data::file::FileOrFileLike;
use crate::data::list_entry::ListEntry;
use crate::data::value::Value;
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

#[pyclass]
pub struct AsyncDatasetClient {
    client: Arc<topk_rs::Client>,
    dataset: String,
}

impl AsyncDatasetClient {
    pub fn new(client: Arc<topk_rs::Client>, dataset: String) -> Self {
        Self { client, dataset }
    }
}

#[pymethods]
impl AsyncDatasetClient {
    #[pyo3(signature = (doc_id, input, metadata))]
    pub fn upsert_file(
        &self,
        py: Python<'_>,
        doc_id: String,
        input: FileOrFileLike,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        let input_file: InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        future_into_py(py, async move {
            let response = client
                .dataset(&dataset)
                .upsert_file(doc_id, input_file, metadata)
                .await
                .map_err(RustError)?;

            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    Ok(UpsertResponse {
                        handle: inner.handle,
                    })
                })
            })
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (ids, fields=None))]
    pub fn get_metadata(
        &self,
        py: Python<'_>,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();

        future_into_py(py, async move {
            let response = client
                .dataset(&dataset)
                .get_metadata(ids, fields)
                .await
                .map_err(RustError)?;
            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    let docs: HashMap<String, HashMap<String, Value>> = inner
                        .docs
                        .into_iter()
                        .map(|(id, doc)| {
                            (
                                id,
                                doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
                            )
                        })
                        .collect();
                    Ok(GetMetadataResponse { docs })
                })
            })
        })
        .map(|result| result.into())
    }

    pub fn update_metadata(
        &self,
        py: Python<'_>,
        doc_id: String,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        future_into_py(py, async move {
            let response = client
                .dataset(&dataset)
                .update_metadata(doc_id, metadata)
                .await
                .map_err(RustError)?;
            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    Ok(UpdateMetadataResponse {
                        handle: inner.handle,
                    })
                })
            })
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, doc_id: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();

        future_into_py(py, async move {
            let response = client
                .dataset(&dataset)
                .delete(doc_id)
                .await
                .map_err(RustError)?;
            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    Ok(DeleteFileResponse {
                        handle: inner.handle,
                    })
                })
            })
        })
        .map(|result| result.into())
    }

    pub fn check_handle(&self, py: Python<'_>, handle: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();

        future_into_py(py, async move {
            Ok(client
                .dataset(&dataset)
                .check_handle(&handle)
                .await
                .map_err(RustError)?)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (handle, config=None))]
    pub fn wait_for_handle(
        &self,
        py: Python<'_>,
        handle: String,
        config: Option<NativeWaitConfig>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        let wait_config = config.map(|c| c.config.into());

        future_into_py(py, async move {
            client
                .dataset(&dataset)
                .wait_for_handle(&handle, wait_config)
                .await
                .map_err(RustError)?;
            Python::attach(|py| Ok(py.None().into_any()))
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (fields=None, filter=None))]
    pub fn list(
        &self,
        _py: Python<'_>,
        fields: Option<Vec<String>>,
        filter: Option<LogicalExpr>,
    ) -> PyResult<AsyncDatasetListIterator> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let filter = filter.map(|f| f.into());

        pyo3_async_runtimes::tokio::get_runtime().spawn({
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

        Ok(AsyncDatasetListIterator {
            receiver: Arc::new(Mutex::new(rx)),
        })
    }
}

#[pyclass]
pub struct AsyncDatasetListIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<ListEntry>>>>,
}

#[pymethods]
impl AsyncDatasetListIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRefMut<'_, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let receiver = slf.receiver.clone();
        future_into_py(py, async move {
            let mut receiver = receiver.lock().await;
            match receiver.recv().await.transpose() {
                Ok(Some(entry)) => Ok(entry),
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Stream exhausted",
                )),
                Err(e) => Err(e),
            }
        })
    }
}
