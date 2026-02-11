use std::{collections::HashMap, sync::Arc};

use pyo3::prelude::*;

use crate::client::sync::runtime::Runtime;
use crate::data::file::FileOrFileLike;
use crate::data::value::Value;
use crate::error::RustError;

#[pyclass]
pub struct DatasetClient {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
    dataset: String,
}

impl DatasetClient {
    pub fn new(runtime: Arc<Runtime>, client: Arc<topk_rs::Client>, dataset: String) -> Self {
        Self {
            runtime,
            client,
            dataset,
        }
    }
}

#[pymethods]
impl DatasetClient {
    #[pyo3(signature = (file_id, input, metadata))]
    pub fn upsert_file(
        &self,
        py: Python<'_>,
        file_id: String,
        input: FileOrFileLike,
        metadata: HashMap<String, Value>,
    ) -> PyResult<String> {
        let input_file: topk_rs::proto::v1::ctx::file::InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let handle = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .upsert_file(file_id, input_file, metadata),
            )
            .map_err(RustError)?;

        Ok(handle.into())
    }

    pub fn get_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
    ) -> PyResult<HashMap<String, Value>> {
        let metadata = self
            .runtime
            .block_on(py, self.client.dataset(&self.dataset).get_metadata(file_id))
            .map_err(RustError)?;

        Ok(metadata.into_iter().map(|(k, v)| (k, v.into())).collect())
    }

    pub fn update_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
        metadata: HashMap<String, Value>,
    ) -> PyResult<String> {
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let handle = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .update_metadata(file_id, metadata),
            )
            .map_err(RustError)?;

        Ok(handle.into())
    }

    pub fn delete(&self, py: Python<'_>, file_id: String) -> PyResult<String> {
        let handle = self
            .runtime
            .block_on(py, self.client.dataset(&self.dataset).delete(file_id))
            .map_err(RustError)?;

        Ok(handle.into())
    }

    pub fn check_handle(&self, py: Python<'_>, handle: String) -> PyResult<bool> {
        let processed = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .check_handle(handle.into()),
            )
            .map_err(RustError)?;

        Ok(processed)
    }
}
