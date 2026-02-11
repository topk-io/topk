use std::sync::Arc;

use futures_util::StreamExt;
use pyo3::prelude::*;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::client::ASK_CHANNEL_BUFFER_SIZE;
use crate::data::ask::{AskResponseMessage, Effort, Sources};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

use super::runtime::Runtime;

#[pyclass]
pub struct AskIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<AskResponseMessage>>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

#[pymethods]
impl AskIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<AskResponseMessage>> {
        self.runtime.block_on(py, async {
            // PyO3 maps Ok(None) from __next__ to raise StopIteration to signal exhaustion, so the loop ends
            self.receiver.recv().await.transpose()
        })
    }
}

impl Drop for AskIterator {
    fn drop(&mut self) {
        // Abort the background task when the iterator is dropped
        self.handle.abort();
    }
}

pub fn ask_stream(
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    effort: Effort,
) -> PyResult<AskIterator> {
    let (tx, rx) = mpsc::channel(ASK_CHANNEL_BUFFER_SIZE);

    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let effort = Some(effort.into());

    // Spawn a task to consume the stream
    let handle = runtime.spawn(async move {
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

    Ok(AskIterator {
        runtime,
        receiver: rx,
        handle,
    })
}

pub fn ask(
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    effort: Effort,
) -> PyResult<AskResponseMessage> {
    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let effort = Some(effort.into());

    runtime.block_on(py, async move {
        let mut stream = client
            .ask(query, sources, filter, effort)
            .await
            .map_err(|e| RustError(e))?;

        let mut last_message: Option<AskResponseMessage> = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    last_message = msg.message.map(|m| m.into());
                }
                Err(e) => {
                    return Err(RustError(e.into()).into());
                }
            }
        }

        last_message.ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Stream ended without any messages")
        })
    })
}
