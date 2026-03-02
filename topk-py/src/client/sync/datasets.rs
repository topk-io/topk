use std::sync::Arc;

use pyo3::prelude::*;

use crate::client::sync::runtime::Runtime;
use crate::client::{
    into_py_response, CreateDatasetResponse, DeleteDatasetResponse, GetDatasetResponse,
    ListDatasetsResponse,
};
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
    pub fn get(&self, py: Python<'_>, dataset_name: String) -> PyResult<Py<GetDatasetResponse>> {
        let response = self
            .runtime
            .block_on(py, self.client.datasets().get(&dataset_name))
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            let dataset: Dataset = inner
                .dataset
                .ok_or(topk_rs::Error::InvalidProto)
                .map_err(RustError)?
                .into();
            Ok(GetDatasetResponse { dataset })
        })
    }

    pub fn list(&self, py: Python<'_>) -> PyResult<Py<ListDatasetsResponse>> {
        let response = self
            .runtime
            .block_on(py, self.client.datasets().list())
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            let datasets: Vec<Dataset> = inner.datasets.into_iter().map(|i| i.into()).collect();
            Ok(ListDatasetsResponse { datasets })
        })
    }

    pub fn create(
        &self,
        py: Python<'_>,
        dataset_name: String,
    ) -> PyResult<Py<CreateDatasetResponse>> {
        let response = self
            .runtime
            .block_on(py, self.client.datasets().create(&dataset_name))
            .map_err(RustError)?;
        into_py_response(py, response, |inner| {
            let dataset: Dataset = inner
                .dataset
                .ok_or(topk_rs::Error::InvalidProto)
                .map_err(RustError)?
                .into();
            Ok(CreateDatasetResponse { dataset })
        })
    }

    pub fn delete(
        &self,
        py: Python<'_>,
        dataset_name: String,
    ) -> PyResult<Py<DeleteDatasetResponse>> {
        let response = self
            .runtime
            .block_on(py, self.client.datasets().delete(&dataset_name))
            .map_err(RustError)?;
        into_py_response(py, response, |_inner| Ok(DeleteDatasetResponse))
    }
}
