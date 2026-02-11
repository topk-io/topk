use std::{collections::HashMap, sync::Arc};

use pyo3::{prelude::*, types::PyAny};
use pyo3_async_runtimes::tokio::future_into_py;

use crate::data::file::FileOrFileLike;
use crate::data::value::Value;
use crate::error::RustError;

#[pyclass]
pub struct AsyncDatasetClient {
    client: Arc<topk_rs::Client>,
    dataset: String,
}

impl AsyncDatasetClient {
    pub fn new(client: Arc<topk_rs::Client>, dataset: String) -> Self {
        Self { client, dataset }
    }
}

#[pymethods]
impl AsyncDatasetClient {
    #[pyo3(signature = (file_id, input, metadata))]
    pub fn upsert_file(
        &self,
        py: Python<'_>,
        file_id: String,
        input: FileOrFileLike,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        let input_file: topk_rs::proto::v1::ctx::file::InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        future_into_py(py, async move {
            let handle: String = client
                .dataset(&dataset)
                .upsert_file(file_id, input_file, metadata)
                .await
                .map_err(RustError)?
                .into();
            Ok(handle)
        })
        .map(|result| result.into())
    }

    pub fn get_metadata(&self, py: Python<'_>, file_id: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();

        future_into_py(py, async move {
            let metadata = client
                .dataset(&dataset)
                .get_metadata(file_id)
                .await
                .map_err(RustError)?;

            let metadata: HashMap<String, Value> =
                metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

            Ok(metadata)
        })
        .map(|result| result.into())
    }

    pub fn update_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        future_into_py(py, async move {
            let handle = client
                .dataset(&dataset)
                .update_metadata(file_id, metadata)
                .await
                .map_err(RustError)?;
            let handle_string: String = handle.into();
            Ok(handle_string)
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, file_id: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();

        future_into_py(py, async move {
            let handle: String = client
                .dataset(&dataset)
                .delete(file_id)
                .await
                .map_err(RustError)?
                .into();
            Ok(handle)
        })
        .map(|result| result.into())
    }

    pub fn check_handle(&self, py: Python<'_>, handle: String) -> PyResult<Py<PyAny>> {
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        future_into_py(py, async move {
            let processed = client
                .dataset(&dataset)
                .check_handle(handle.into())
                .await
                .map_err(RustError)?;
            Ok(processed)
        })
        .map(|result| result.into())
    }
}
