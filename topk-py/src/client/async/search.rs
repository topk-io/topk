use std::sync::Arc;

use futures_util::StreamExt;
use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::client::ASK_CHANNEL_BUFFER_SIZE;
use crate::data::ask::{SearchResult, Sources};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

#[pyclass]
pub struct AsyncSearchIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<SearchResult>>>>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

#[pymethods]
impl AsyncSearchIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRefMut<'_, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let receiver = slf.receiver.clone();
        future_into_py(py, async move {
            let mut receiver = receiver.lock().await;
            match receiver.recv().await.transpose() {
                Ok(Some(msg)) => Ok(msg),
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Stream exhausted",
                )),
                Err(e) => Err(e),
            }
        })
    }
}

impl Drop for AsyncSearchIterator {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub fn search_stream(
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    top_k: u32,
    select_fields: Option<Vec<String>>,
) -> PyResult<Py<AsyncSearchIterator>> {
    let (tx, rx) = mpsc::channel(ASK_CHANNEL_BUFFER_SIZE);

    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let select_fields = select_fields.unwrap_or_default();

    let handle = pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
        let mut stream = match client
            .search(query, sources, top_k, filter, select_fields)
            .await
        {
            Ok(response) => response.into_inner(),
            Err(e) => {
                let _ = tx.send(Err(RustError(e).into())).await;
                return;
            }
        };

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => match msg.try_into() {
                    Ok(sr) => {
                        if let Err(mpsc::error::SendError(_)) = tx.send(Ok(sr)).await {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err::<SearchResult, PyErr>(PyErr::from(e))).await;
                        break;
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(RustError(e.into()).into())).await;
                    break;
                }
            }
        }
    });

    Ok(Py::new(
        py,
        AsyncSearchIterator {
            receiver: Arc::new(tokio::sync::Mutex::new(rx)),
            handle,
        },
    )?
    .into())
}

pub fn search(
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    top_k: u32,
    select_fields: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let select_fields = select_fields.unwrap_or_default();

    future_into_py(py, async move {
        let mut stream = client
            .search(query, sources, top_k, filter, select_fields)
            .await
            .map_err(RustError)?
            .into_inner();

        let mut results: Vec<SearchResult> = Vec::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => results.push(msg.try_into()?),
                Err(e) => return Err(RustError(e.into()).into()),
            }
        }
        Ok(results)
    })
    .map(|result| result.into())
}
