use std::sync::Arc;

use futures_util::StreamExt;
use pyo3::prelude::*;
use tokio::sync::mpsc;
use topk_rs::proto::v1::ctx::ask_result::Message;

use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::ask::{AskResult, Mode, Sources};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

use super::runtime::Runtime;

#[pyclass]
pub struct AskIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<AskResult>>,
}

#[pymethods]
impl AskIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<AskResult>> {
        self.runtime.block_on(py, async {
            // PyO3 maps Ok(None) from __next__ to raise StopIteration to signal exhaustion, so the loop ends
            self.receiver.recv().await.transpose()
        })
    }
}

pub fn ask_stream(
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    mode: Option<Mode>,
    select_fields: Option<Vec<String>>,
) -> PyResult<AskIterator> {
    let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let mode = mode.map(|m| m.into());

    // Spawn a task to consume the stream
    runtime.spawn(async move {
        let mut stream = match client
            .ask_stream(query, sources, filter, mode, select_fields)
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

    Ok(AskIterator {
        runtime,
        receiver: rx,
    })
}

pub fn ask(
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    mode: Option<Mode>,
    select_fields: Option<Vec<String>>,
) -> PyResult<AskResult> {
    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let mode = mode.map(|m| m.into());

    runtime.block_on(py, async move {
        let answer = client
            .ask(query, sources, filter, mode, select_fields)
            .await
            .map_err(RustError)?
            .into_inner();

        AskResult::try_from(Message::Answer(answer)).map_err(Into::<PyErr>::into)
    })
}
