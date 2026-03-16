use std::sync::Arc;

use futures_util::{StreamExt, TryStreamExt};
use pyo3::prelude::*;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::data::ask::{SearchResult, Sources};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

use super::runtime::Runtime;

const CHANNEL_BUFFER_SIZE: usize = 32;

#[pyclass]
pub struct SearchIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<SearchResult>>,
    #[allow(dead_code)]
    handle: JoinHandle<()>,
}

#[pymethods]
impl SearchIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<SearchResult>> {
        self.runtime
            .block_on(py, async { self.receiver.recv().await.transpose() })
    }
}

impl Drop for SearchIterator {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub fn search_stream(
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    top_k: u32,
    select_fields: Option<Vec<String>>,
) -> PyResult<SearchIterator> {
    let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let select_fields = select_fields.unwrap_or_default();

    let handle = runtime.spawn(async move {
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

    Ok(SearchIterator {
        runtime,
        receiver: rx,
        handle,
    })
}

pub fn search(
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    sources: Sources,
    filter: Option<LogicalExpr>,
    top_k: u32,
    select_fields: Option<Vec<String>>,
) -> PyResult<Vec<SearchResult>> {
    let sources = sources.into_iter();
    let filter = filter.map(|f| f.into());
    let select_fields = select_fields.unwrap_or_default();

    runtime.block_on(py, async move {
        let stream = client
            .search(query, sources, top_k, filter, select_fields)
            .await
            .map_err(RustError)?
            .into_inner();

        stream
            .map_err(|e| PyErr::from(RustError(e.into())))
            .and_then(|msg| {
                std::future::ready(SearchResult::try_from(msg).map_err(Into::<PyErr>::into))
            })
            .try_collect::<Vec<_>>()
            .await
    })
}
