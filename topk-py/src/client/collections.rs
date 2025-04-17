use crate::client::Runtime;
use crate::data::collection::Collection;
use crate::error::RustError;
use crate::schema::field_spec::FieldSpec;
use pyo3::prelude::*;
use std::{collections::HashMap, sync::Arc};
use topk_protos::v1::control::FieldSpec as FieldSpecPb;

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

    pub fn list(&self, py: Python<'_>) -> PyResult<Vec<Collection>> {
        let collections = self
            .runtime
            .block_on(py, self.client.collections().list())
            .map_err(RustError)?;

        Ok(collections.into_iter().map(|i| i.into()).collect())
    }

    pub fn create(
        &self,
        py: Python<'_>,
        collection_name: String,
        schema: HashMap<String, FieldSpec>,
    ) -> PyResult<Collection> {
        let collection = self
            .runtime
            .block_on(
                py,
                self.client.collections().create(
                    &collection_name,
                    schema
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect::<HashMap<String, FieldSpecPb>>(),
                ),
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
