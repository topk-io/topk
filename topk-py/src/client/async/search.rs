use std::sync::Arc;

use futures_util::{StreamExt, TryStreamExt};
use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, Mutex};

use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::ask::{SearchResult, Source};
use crate::error::RustError;
use crate::expr::logical::LogicalExpr;

#[pyclass]
pub struct AsyncSearchIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<SearchResult>>>>,
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

pub fn search_stream(
    client: Arc<topk_rs::Client>,
    query: String,
    datasets: Vec<Source>,
    filter: Option<LogicalExpr>,
    top_k: u32,
    select_fields: Option<Vec<String>>,
) -> PyResult<AsyncSearchIterator> {
    let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

    let filter = filter.map(|f| f.into());
    let select_fields = select_fields.unwrap_or_default();

    pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
        let mut stream = match client
            .search(query, datasets, top_k, filter, select_fields)
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

    Ok(AsyncSearchIterator {
        receiver: Arc::new(tokio::sync::Mutex::new(rx)),
    })
}

pub fn search(
    client: Arc<topk_rs::Client>,
    py: Python<'_>,
    query: String,
    datasets: Vec<Source>,
    filter: Option<LogicalExpr>,
    top_k: u32,
    select_fields: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let filter = filter.map(|f| f.into());
    let select_fields = select_fields.unwrap_or_default();

    future_into_py(py, async move {
        let stream = client
            .search(query, datasets, top_k, filter, select_fields)
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
    .map(|result| result.into())
}
