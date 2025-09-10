use crate::error::RustError;
use crate::schema::field_spec::FieldSpec;
use crate::{data::collection::Collection, schema::Schema};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::{collections::HashMap, sync::Arc};

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
    pub fn get(&self, py: Python<'_>, collection_name: String) -> PyResult<PyObject> {
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

    pub fn list(&self, py: Python<'_>) -> PyResult<PyObject> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let collections = client.collections().list().await.map_err(RustError)?;

            Ok(collections
                .into_iter()
                .map(|i| i.into())
                .collect::<Vec<Collection>>())
        })
        .map(|result| result.into())
    }

    pub fn create(
        &self,
        py: Python<'_>,
        collection_name: String,
        schema: HashMap<String, FieldSpec>,
    ) -> PyResult<PyObject> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let schema = Schema(schema);

            let collection = client
                .collections()
                .create(&collection_name, schema)
                .await
                .map_err(RustError)?;

            let collection: Collection = collection.into();

            Ok(collection)
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, collection_name: String) -> PyResult<PyObject> {
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
