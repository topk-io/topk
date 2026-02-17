use std::sync::Arc;

use futures_util::StreamExt;
use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use crate::client::ASK_CHANNEL_BUFFER_SIZE;
use crate::data::ask::{AskResponseMessage, Effort, Sources};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

#[pyclass]
pub struct AsyncAskIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<AskResponseMessage>>>>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

#[pymethods]
impl AsyncAskIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRefMut<'_, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let receiver = slf.receiver.clone();
        future_into_py(py, async move {
            let mut receiver = receiver.lock().await;
            match receiver.recv().await.transpose() {
                Ok(Some(msg)) => Ok(msg),
                // Channel exhausted: raise StopAsyncIteration so `async for` terminates
                // (returning Ok(None) would yield None indefinitely instead of ending the loop)
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Stream exhausted",
                )),
                Err(e) => Err(e),
            }
        })
    }
}

impl Drop for AsyncAskIterator {
    fn drop(&mut self) {
        // Abort the background task when the iterator is dropped
        self.handle.abort();
    }
}

pub fn ask_stream(
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    effort: Effort,
) -> PyResult<Py<AsyncAskIterator>> {
    let (tx, rx) = mpsc::channel(ASK_CHANNEL_BUFFER_SIZE);

    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let effort = Some(effort.into());

    let handle = pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
        let mut stream = match client.ask(query, sources, filter, effort).await {
            Ok(stream) => stream,
            Err(e) => {
                let _ = tx.send(Err(RustError(e).into())).await;
                return;
            }
        };

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => match msg.message {
                    Some(inner) => {
                        if let Err(mpsc::error::SendError(_)) = tx.send(Ok(inner.into())).await {
                            // Channel closed: receiver dropped, Python stopped iterating.
                            break;
                        }
                    }
                    None => {
                        let _ = tx
                            .send(Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                "AskResponseMessage has no message",
                            )))
                            .await;
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
        AsyncAskIterator {
            receiver: Arc::new(tokio::sync::Mutex::new(rx)),
            handle,
        },
    )?
    .into())
}

pub fn ask(
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    effort: Effort,
) -> PyResult<Py<PyAny>> {
    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let effort = Some(effort.into());

    future_into_py(py, async move {
        let mut stream = client
            .ask(query, sources.into_iter(), filter, effort)
            .await
            .map_err(|e| RustError(e))?;

        let mut last_message: Option<AskResponseMessage> = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => match msg.message {
                    Some(inner) => last_message = Some(inner.into()),
                    None => {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                            "Invalid proto: AskResponseMessage has no message",
                        ))
                    }
                },
                Err(e) => {
                    return Err(RustError(e.into()).into());
                }
            }
        }

        last_message.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Stream ended without any messages")
        })
    })
    .map(|result| result.into())
}
