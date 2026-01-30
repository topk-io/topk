use std::sync::Arc;

use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;

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
            let dataset: Dataset = client
                .datasets()
                .get(&dataset_name)
                .await
                .map_err(RustError)?
                .into();

            Ok(dataset)
        })
        .map(|result| result.into())
    }

    pub fn list(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let datasets: Vec<Dataset> = client
                .datasets()
                .list()
                .await
                .map_err(RustError)?
                .into_iter()
                .map(|i| i.into())
                .collect();

            Ok(datasets)
        })
        .map(|result| result.into())
    }

    pub fn create(&self, py: Python<'_>, dataset_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            let dataset: Dataset = client
                .datasets()
                .create(&dataset_name)
                .await
                .map_err(RustError)?
                .into();

            Ok(dataset)
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, dataset_name: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();

        future_into_py(py, async move {
            client
                .datasets()
                .delete(&dataset_name)
                .await
                .map_err(RustError)?;
            Ok(())
        })
        .map(|result| result.into())
    }
}
