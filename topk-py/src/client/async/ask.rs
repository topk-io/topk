use std::sync::Arc;

use futures_util::StreamExt;
use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, Mutex};

use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::ask::{AskResult, Mode, Source};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

#[pyclass]
pub struct AsyncAskIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<AskResult>>>>,
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
                // Channel exhausted: raise StopAsyncIteration so `async for` in python terminates
                // (returning Ok(None) would yield None indefinitely instead of ending the loop)
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Stream exhausted",
                )),
                Err(e) => Err(e),
            }
        })
    }
}

pub fn ask(
    client: Arc<topk_rs::Client>,
    query: String,
    datasets: Vec<Source>,
    filter: Option<LogicalExpr>,
    mode: Option<Mode>,
    select_fields: Option<Vec<String>>,
    include_content: Option<bool>,
) -> PyResult<AsyncAskIterator> {
    let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    let filter = filter.map(|f| f.into());
    let mode = mode.map(|m| m.into());

    pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
        let mut stream = match client
            .ask(
                query,
                datasets,
                filter,
                mode,
                select_fields,
                include_content,
            )
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
                Ok(msg) => match msg.message {
                    Some(inner) => match inner.try_into() {
                        Ok(msg) => {
                            if let Err(mpsc::error::SendError(_)) = tx.send(Ok(msg)).await {
                                // Channel closed: receiver dropped, Python stopped iterating.
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err::<AskResult, PyErr>(PyErr::from(e))).await;
                            break;
                        }
                    },
                    None => {
                        let _ = tx
                            .send(Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                "Invalid proto: AskResult has no message",
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

    Ok(AsyncAskIterator {
        receiver: Arc::new(tokio::sync::Mutex::new(rx)),
    })
}
