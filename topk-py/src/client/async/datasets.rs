use std::sync::Arc;

use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;

use crate::client::{
    into_py_response, CreateDatasetResponse, DeleteDatasetResponse, GetDatasetResponse,
    ListDatasetsResponse,
};
use crate::data::dataset::Dataset;
use crate::error::RustError;

#[pyclass]
pub struct AsyncDatasetsClient {
    client: Arc<topk_rs::Client>,
}

impl AsyncDatasetsClient {
    pub fn new(client: Arc<topk_rs::Client>) -> Self {
        Self { client }
    }
}

#[pymethods]
impl AsyncDatasetsClient {
    pub fn get(&self, py: Python<'_>, dataset_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let response = client
                .datasets()
                .get(&dataset_name)
                .await
                .map_err(RustError)?;
            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    let dataset: Dataset = inner
                        .dataset
                        .ok_or(topk_rs::Error::InvalidProto)
                        .map_err(RustError)?
                        .into();
                    Ok(GetDatasetResponse { dataset })
                })
            })
        })
        .map(|result| result.into())
    }

    pub fn list(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let response = client.datasets().list().await.map_err(RustError)?;
            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    let datasets: Vec<Dataset> =
                        inner.datasets.into_iter().map(|i| i.into()).collect();
                    Ok(ListDatasetsResponse { datasets })
                })
            })
        })
        .map(|result| result.into())
    }

    pub fn create(&self, py: Python<'_>, dataset_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let response = client
                .datasets()
                .create(&dataset_name, None)
                .await
                .map_err(RustError)?;
            Python::attach(|py| {
                into_py_response(py, response, |inner| {
                    let dataset: Dataset = inner
                        .dataset
                        .ok_or(topk_rs::Error::InvalidProto)
                        .map_err(RustError)?
                        .into();
                    Ok(CreateDatasetResponse { dataset })
                })
            })
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, dataset_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let response = client
                .datasets()
                .delete(&dataset_name)
                .await
                .map_err(RustError)?;
            Python::attach(|py| into_py_response(py, response, |_inner| Ok(DeleteDatasetResponse)))
        })
        .map(|result| result.into())
    }
}
