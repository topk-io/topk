use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;
use tokio::sync::{mpsc, Mutex};

use crate::client::CHANNEL_BUFFER_SIZE;
use crate::error::RustError;
use crate::schema::SchemaFieldSpec;
use crate::{data::collection::Collection, schema::Schema};

#[pyclass]
pub struct AsyncCollectionsClient {
    client: Arc<topk_rs::Client>,
}

impl AsyncCollectionsClient {
    pub fn new(client: Arc<topk_rs::Client>) -> Self {
        Self { client }
    }
}

#[pymethods]
impl AsyncCollectionsClient {
    pub fn get(&self, py: Python<'_>, collection_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let collection = client
                .collections()
                .get(&collection_name)
                .await
                .map_err(RustError)?;

            let collection: Collection = collection.into();

            Ok(collection)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (prefix=None))]
    pub fn list(&self, prefix: Option<String>) -> PyResult<AsyncCollectionListIterator> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let client = self.client.clone();

        pyo3_async_runtimes::tokio::get_runtime().spawn(async move {
            let mut stream = match client.collections().list(prefix).await {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(RustError(e).into())).await;
                    return;
                }
            };

            while let Some(result) = stream.next().await {
                match result {
                    Ok(collection) => {
                        if let Err(mpsc::error::SendError(_)) =
                            tx.send(Ok(Collection::from(collection))).await
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(RustError(e).into())).await;
                        break;
                    }
                }
            }
        });

        Ok(AsyncCollectionListIterator {
            receiver: Arc::new(Mutex::new(rx)),
        })
    }

    pub fn create(
        &self,
        py: Python<'_>,
        collection_name: String,
        schema: HashMap<String, SchemaFieldSpec>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let schema = Schema(schema.into_iter().map(|(k, v)| (k, v.0)).collect());

            let collection = client
                .collections()
                .create(&collection_name, schema, None)
                .await
                .map_err(RustError)?;

            let collection: Collection = collection.into();

            Ok(collection)
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, collection_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            client
                .collections()
                .delete(&collection_name)
                .await
                .map_err(RustError)?;

            Ok(())
        })
        .map(|result| result.into())
    }
}

#[pyclass]
pub struct AsyncCollectionListIterator {
    receiver: Arc<Mutex<mpsc::Receiver<PyResult<Collection>>>>,
}

#[pymethods]
impl AsyncCollectionListIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRefMut<'_, Self>, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let receiver = slf.receiver.clone();
        future_into_py(py, async move {
            let mut receiver = receiver.lock().await;
            match receiver.recv().await.transpose() {
                Ok(Some(collection)) => Ok(collection),
                Ok(None) => Err(pyo3::exceptions::PyStopAsyncIteration::new_err(
                    "Stream exhausted",
                )),
                Err(e) => Err(e),
            }
        })
    }
}
