use std::{collections::HashMap, sync::Arc};

use pyo3::prelude::*;

use crate::client::sync::runtime::Runtime;
use crate::client::{
    into_py_response, CheckHandleResponse, DeleteFileResponse, GetMetadataResponse,
    UpdateMetadataResponse, UpsertFileResponse,
};
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
    ) -> PyResult<Py<UpsertFileResponse>> {
        let input_file: topk_rs::proto::v1::ctx::file::InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .upsert_file(file_id, input_file, metadata),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(UpsertFileResponse {
                handle: inner.handle,
            })
        })
    }

    pub fn get_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
        fields: Option<Vec<String>>,
    ) -> PyResult<Py<GetMetadataResponse>> {
        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .get_metadata(file_id, fields),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            let metadata: HashMap<String, Value> = inner
                .metadata
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect();
            Ok(GetMetadataResponse { metadata })
        })
    }

    pub fn update_metadata(
        &self,
        py: Python<'_>,
        file_id: String,
        metadata: HashMap<String, Value>,
    ) -> PyResult<Py<UpdateMetadataResponse>> {
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .update_metadata(file_id, metadata),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(UpdateMetadataResponse {
                handle: inner.handle,
            })
        })
    }

    pub fn delete(&self, py: Python<'_>, file_id: String) -> PyResult<Py<DeleteFileResponse>> {
        let response = self
            .runtime
            .block_on(py, self.client.dataset(&self.dataset).delete(file_id))
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(DeleteFileResponse {
                handle: inner.handle,
            })
        })
    }

    pub fn check_handle(
        &self,
        py: Python<'_>,
        handle: String,
    ) -> PyResult<Py<CheckHandleResponse>> {
        let response = self
            .runtime
            .block_on(
                py,
                self.client
                    .dataset(&self.dataset)
                    .check_handle(handle.into()),
            )
            .map_err(RustError)?;

        into_py_response(py, response, |inner| {
            Ok(CheckHandleResponse {
                processed: inner.processed,
            })
        })
    }
}
