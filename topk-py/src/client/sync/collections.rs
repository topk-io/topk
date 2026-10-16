use std::{collections::HashMap, sync::Arc};

use futures_util::StreamExt;
use pyo3::prelude::*;
use tokio::sync::mpsc;

use crate::client::CHANNEL_BUFFER_SIZE;
use crate::data::collection::Collection;
use crate::error::RustError;
use crate::schema::SchemaFieldSpec;
use crate::{client::sync::runtime::Runtime, schema::Schema};

#[pyclass]
pub struct CollectionsClient {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
}

impl CollectionsClient {
    pub fn new(runtime: Arc<Runtime>, client: Arc<topk_rs::Client>) -> Self {
        Self { runtime, client }
    }
}

#[pymethods]
impl CollectionsClient {
    pub fn get(&self, py: Python<'_>, collection_name: String) -> PyResult<Collection> {
        let collection = self
            .runtime
            .block_on(py, self.client.collections().get(&collection_name))
            .map_err(RustError)?;

        Ok(collection.into())
    }

    #[pyo3(signature = (prefix=None))]
    pub fn list(&self, prefix: Option<String>) -> PyResult<CollectionListIterator> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);

        self.runtime.spawn({
            let client = self.client.clone();
            async move {
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
                                tx.send(Ok(collection.into())).await
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
            }
        });

        Ok(CollectionListIterator {
            runtime: self.runtime.clone(),
            receiver: rx,
        })
    }

    pub fn create(
        &self,
        py: Python<'_>,
        collection_name: String,
        schema: HashMap<String, SchemaFieldSpec>,
    ) -> PyResult<Collection> {
        let schema = Schema(schema.into_iter().map(|(k, v)| (k, v.0)).collect());

        let collection = self
            .runtime
            .block_on(
                py,
                self.client
                    .collections()
                    .create(&collection_name, schema, None),
            )
            .map_err(RustError)?;

        Ok(collection.into())
    }

    pub fn delete(&self, py: Python<'_>, collection_name: String) -> PyResult<()> {
        Ok(self
            .runtime
            .block_on(py, self.client.collections().delete(&collection_name))
            .map_err(RustError)?)
    }
}

#[pyclass]
pub struct CollectionListIterator {
    runtime: Arc<Runtime>,
    receiver: mpsc::Receiver<PyResult<Collection>>,
}

#[pymethods]
impl CollectionListIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self, py: Python<'_>) -> PyResult<Option<Collection>> {
        self.runtime
            .block_on(py, async { self.receiver.recv().await.transpose() })
    }
}
