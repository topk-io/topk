use std::sync::Arc;

use pyo3::prelude::*;

use crate::client::sync::runtime::Runtime;
use crate::data::dataset::Dataset;
use crate::error::RustError;

#[pyclass]
pub struct DatasetsClient {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
}

impl DatasetsClient {
    pub fn new(runtime: Arc<Runtime>, client: Arc<topk_rs::Client>) -> Self {
        Self { runtime, client }
    }
}

#[pymethods]
impl DatasetsClient {
    pub fn get(&self, py: Python<'_>, dataset_name: String) -> PyResult<Dataset> {
        let dataset = self
            .runtime
            .block_on(py, self.client.datasets().get(&dataset_name))
            .map_err(RustError)?;

        Ok(dataset.into())
    }

    pub fn list(&self, py: Python<'_>) -> PyResult<Vec<Dataset>> {
        let datasets = self
            .runtime
            .block_on(py, self.client.datasets().list())
            .map_err(RustError)?;

        Ok(datasets.into_iter().map(|i| i.into()).collect())
    }

    pub fn create(&self, py: Python<'_>, dataset_name: String) -> PyResult<Dataset> {
        let dataset = self
            .runtime
            .block_on(py, self.client.datasets().create(&dataset_name))
            .map_err(RustError)?;

        Ok(dataset.into())
    }

    pub fn delete(&self, py: Python<'_>, dataset_name: String) -> PyResult<()> {
        Ok(self
            .runtime
            .block_on(py, self.client.datasets().delete(&dataset_name))
            .map_err(RustError)?)
    }
}
