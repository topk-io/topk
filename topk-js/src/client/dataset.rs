use std::collections::HashMap;

use futures_util::TryStreamExt;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;

use crate::data::NativeValue;
use crate::data::Value;
use crate::error::TopkError;
use crate::expr::logical::LogicalExpression;
use crate::utils::{js_object, js_set};

/// Input for upserting a file to a dataset.
///
/// Provide either a `path` to a file on disk, or inline data via `data` + `fileName` + `mimeType`.
#[napi(object)]
pub struct FileInput {
    /// Path to a file on disk. If provided, fileName and mimeType are inferred.
    pub path: Option<String>,
    /// Inline file data as a Buffer.
    pub data: Option<Buffer>,
    /// File name (required when using inline `data`).
    pub file_name: Option<String>,
    /// MIME type (required when using inline `data`).
    pub mime_type: Option<String>,
}

/// Configuration for waiting on a handle to be processed.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct WaitConfig {
    /// How often to poll for the handle status, in seconds. Default is 5.
    pub frequency_secs: Option<u32>,
    /// Maximum time to wait before timing out, in seconds. Default is 300.
    pub timeout_secs: Option<u32>,
}

impl From<WaitConfig> for topk_rs::client::WaitConfig {
    fn from(config: WaitConfig) -> Self {
        topk_rs::client::WaitConfig {
            frequency: std::time::Duration::from_secs(
                config.frequency_secs.unwrap_or(5) as u64,
            ),
            timeout: std::time::Duration::from_secs(
                config.timeout_secs.unwrap_or(300) as u64,
            ),
        }
    }
}

/// An entry returned when listing files in a dataset.
pub struct ListEntry {
    pub id: String,
    pub name: String,
    pub size: f64,
    pub mime_type: String,
    pub metadata: HashMap<String, NativeValue>,
}

impl From<topk_rs::proto::v1::ctx::ListEntry> for ListEntry {
    fn from(entry: topk_rs::proto::v1::ctx::ListEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            size: entry.size as f64,
            mime_type: entry.mime_type,
            metadata: entry
                .metadata
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}

impl ToNapiValue for ListEntry {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let obj = js_object(env)?;
        js_set(env, obj, "id", val.id)?;
        js_set(env, obj, "name", val.name)?;
        js_set(env, obj, "size", val.size)?;
        js_set(env, obj, "mimeType", val.mime_type)?;

        let meta = js_object(env)?;
        for (k, v) in val.metadata {
            js_set(env, meta, &k, v)?;
        }
        let key = std::ffi::CString::new("metadata").unwrap();
        napi::check_status!(napi::sys::napi_set_named_property(
            env,
            obj,
            key.as_ptr(),
            meta
        ))?;

        Ok(obj)
    }
}

/// Result of an upsert, delete, or metadata update operation.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HandleResponse {
    /// Handle that can be used to check or wait for processing completion.
    pub handle: String,
}

/// Client for interacting with a specific dataset.
///
/// This client provides methods to perform operations on a specific dataset,
/// including uploading files, managing metadata, and listing entries.
/// @internal
/// @hideconstructor
#[napi]
pub struct DatasetClient {
    client: Arc<topk_rs::Client>,
    dataset: String,
}

#[napi]
impl DatasetClient {
    pub fn new(client: Arc<topk_rs::Client>, dataset: String) -> Self {
        Self { client, dataset }
    }

    /// Uploads a file to the dataset.
    #[napi]
    pub async fn upsert_file(
        &self,
        doc_id: String,
        input: FileInput,
        #[napi(ts_arg_type = "Record<string, any>")] metadata: HashMap<String, Value>,
    ) -> Result<HandleResponse> {
        let input_file = file_input_to_input_file(input)?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .client
            .dataset(&self.dataset)
            .upsert_file(doc_id, input_file, metadata)
            .await
            .map_err(TopkError::from)?;

        Ok(HandleResponse {
            handle: response.into_inner().handle,
        })
    }

    /// Retrieves metadata for documents by their IDs.
    #[napi(ts_return_type = "Promise<Record<string, Record<string, any>>>")]
    pub async fn get_metadata(
        &self,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
    ) -> Result<HashMap<String, HashMap<String, NativeValue>>> {
        let response = self
            .client
            .dataset(&self.dataset)
            .get_metadata(ids, fields)
            .await
            .map_err(TopkError::from)?;

        Ok(response
            .into_inner()
            .docs
            .into_iter()
            .map(|(id, doc)| {
                (
                    id,
                    doc.fields
                        .into_iter()
                        .map(|(k, v)| (k, v.into()))
                        .collect(),
                )
            })
            .collect())
    }

    /// Updates metadata for a document.
    #[napi]
    pub async fn update_metadata(
        &self,
        doc_id: String,
        #[napi(ts_arg_type = "Record<string, any>")] metadata: HashMap<String, Value>,
    ) -> Result<HandleResponse> {
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .client
            .dataset(&self.dataset)
            .update_metadata(doc_id, metadata)
            .await
            .map_err(TopkError::from)?;

        Ok(HandleResponse {
            handle: response.into_inner().handle,
        })
    }

    /// Deletes a document from the dataset.
    #[napi(js_name = "delete")]
    pub async fn delete_doc(&self, doc_id: String) -> Result<HandleResponse> {
        let response = self
            .client
            .dataset(&self.dataset)
            .delete(doc_id)
            .await
            .map_err(TopkError::from)?;

        Ok(HandleResponse {
            handle: response.into_inner().handle,
        })
    }

    /// Checks if a handle has been processed.
    #[napi]
    pub async fn check_handle(&self, handle: String) -> Result<bool> {
        Ok(self
            .client
            .dataset(&self.dataset)
            .check_handle(&handle)
            .await
            .map_err(TopkError::from)?)
    }

    /// Waits for a handle to be processed. Polls periodically until done or timeout.
    #[napi]
    pub async fn wait_for_handle(
        &self,
        handle: String,
        config: Option<WaitConfig>,
    ) -> Result<()> {
        Ok(self
            .client
            .dataset(&self.dataset)
            .wait_for_handle(&handle, config.map(|c| c.into()))
            .await
            .map_err(TopkError::from)?)
    }

    /// Lists files in the dataset.
    #[napi(ts_return_type = "Promise<Array<ListEntry>>")]
    pub async fn list(
        &self,
        fields: Option<Vec<String>>,
        #[napi(ts_arg_type = "query.LogicalExpression | undefined")] filter: Option<
            &LogicalExpression,
        >,
    ) -> Result<Vec<ListEntry>> {
        let filter = filter.map(|f| f.clone().into());

        let response = self
            .client
            .dataset(&self.dataset)
            .list(fields, filter)
            .await
            .map_err(TopkError::from)?;

        response
            .into_inner()
            .map_ok(|entry| entry.into())
            .try_collect::<Vec<ListEntry>>()
            .await
            .map_err(|e| napi::Error::from_reason(format!("stream error: {e}")))
    }
}

fn file_input_to_input_file(
    input: FileInput,
) -> Result<topk_rs::proto::v1::ctx::file::InputFile> {
    use topk_rs::proto::v1::ctx::file::InputFile;

    if let Some(path) = input.path {
        return InputFile::from_path(path)
            .map_err(|e| napi::Error::from_reason(format!("{e}")));
    }

    if let Some(data) = input.data {
        let file_name = input.file_name.ok_or_else(|| {
            napi::Error::from_reason("fileName is required when using inline data")
        })?;
        let mime_type = input.mime_type.ok_or_else(|| {
            napi::Error::from_reason("mimeType is required when using inline data")
        })?;
        return InputFile::from_bytes(file_name, data.to_vec(), mime_type)
            .map_err(|e| napi::Error::from_reason(format!("{e}")));
    }

    Err(napi::Error::from_reason(
        "FileInput must have either `path` or `data`",
    ))
}
