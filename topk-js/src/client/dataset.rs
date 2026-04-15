use std::collections::HashMap;

use futures_util::{StreamExt, TryStreamExt};
use napi::bindgen_prelude::*;
use napi::tokio::{
    self,
    sync::{mpsc, Mutex},
};
use napi_derive::napi;
use std::sync::Arc;

use super::spawn_stream_task;
use super::STREAM_BUFFER_SIZE;
use crate::data::NativeValue;
use crate::data::Value;
use crate::error::TopkError;
use crate::expr::logical::LogicalExpression;

type DatasetListStreamMessage = std::result::Result<ListEntry, String>;

/// Input for upserting a file to a dataset.
///
/// Provide either a `path` to a file on disk, or inline data via `data` + `fileName` + `mimeType`.
#[napi(object)]
pub struct InputFile {
    /// Path to a file on disk. If provided, fileName and mimeType are inferred.
    pub path: Option<String>,
    /// Inline file data as a Buffer.
    pub data: Option<Buffer>,
    /// File name (required when using inline `data`).
    pub file_name: Option<String>,
    /// MIME type (required when using inline `data`).
    pub mime_type: Option<String>,
}

impl TryFrom<InputFile> for topk_rs::proto::v1::ctx::file::InputFile {
    type Error = napi::Error;

    fn try_from(input: InputFile) -> Result<Self> {
        if let Some(path) = input.path {
            return topk_rs::proto::v1::ctx::file::InputFile::from_path(path)
                .map_err(|e| napi::Error::from_reason(format!("{e}")));
        }

        if let Some(data) = input.data {
            let file_name = input.file_name.ok_or_else(|| {
                napi::Error::from_reason("fileName is required when using inline data")
            })?;
            let mime_type = input.mime_type.ok_or_else(|| {
                napi::Error::from_reason("mimeType is required when using inline data")
            })?;
            return topk_rs::proto::v1::ctx::file::InputFile::from_bytes(
                file_name,
                data.to_vec(),
                mime_type,
            )
            .map_err(|e| napi::Error::from_reason(format!("{e}")));
        }

        Err(napi::Error::from_reason(
            "InputFile must have either `path` or `data`",
        ))
    }
}

/// Configuration for polling when waiting for a handle to be processed.
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
            frequency: std::time::Duration::from_secs(config.frequency_secs.unwrap_or(5) as u64),
            timeout: std::time::Duration::from_secs(config.timeout_secs.unwrap_or(300) as u64),
        }
    }
}

/// Entry in a dataset.
#[napi(object, object_from_js = false)]
pub struct ListEntry {
    pub id: String,
    pub name: String,
    pub size: f64,
    pub mime_type: String,
    #[napi(ts_type = "Record<string, any>")]
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

/// Result of an upsert, delete, or metadata update operation.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HandleResponse {
    /// Handle that can be used to check or wait for processing completion.
    pub handle: String,
}

/// Iterator for dataset list responses.
#[napi(async_iterator)]
pub struct DatasetListStream {
    receiver: Arc<Mutex<mpsc::Receiver<DatasetListStreamMessage>>>,
}

impl DatasetListStream {
    fn new(receiver: mpsc::Receiver<DatasetListStreamMessage>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }
}

#[napi]
impl AsyncGenerator for DatasetListStream {
    type Yield = ListEntry;
    type Next = ();
    type Return = ();

    fn next(
        &mut self,
        _value: Option<Self::Next>,
    ) -> impl std::future::Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
        let receiver = self.receiver.clone();
        async move {
            let mut guard = receiver.lock().await;
            match guard.recv().await {
                Some(Ok(v)) => Ok(Some(v)),
                Some(Err(e)) => Err(napi::Error::from_reason(e)),
                None => Ok(None),
            }
        }
    }
}

#[napi]
impl DatasetListStream {
    /// Returns the next entry in the stream.
    #[napi]
    pub async unsafe fn next(&mut self) -> napi::Result<Option<ListEntry>> {
        let mut guard = self.receiver.lock().await;
        match guard.recv().await {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(napi::Error::from_reason(e)),
            None => Ok(None),
        }
    }
}

/// Client for dataset operations.
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

    /// Upsert a file to the dataset.
    #[napi]
    pub async fn upsert_file(
        &self,
        doc_id: String,
        input: InputFile,
        #[napi(ts_arg_type = "Record<string, any>")] metadata: HashMap<String, Value>,
    ) -> Result<HandleResponse> {
        let input: topk_rs::proto::v1::ctx::file::InputFile = input.try_into()?;
        let metadata: HashMap<String, topk_rs::proto::v1::data::Value> =
            metadata.into_iter().map(|(k, v)| (k, v.into())).collect();

        let response = self
            .client
            .dataset(&self.dataset)
            .upsert_file(doc_id, input, metadata)
            .await
            .map_err(TopkError::from)?;

        Ok(HandleResponse {
            handle: response.into_inner().handle,
        })
    }

    /// Get metadata for one or more documents.
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
                    doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
                )
            })
            .collect())
    }

    /// Update metadata for a file.
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

    /// Delete a file from the dataset.
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

    /// Return whether the handle has been processed.
    #[napi]
    pub async fn check_handle(&self, handle: String) -> Result<bool> {
        Ok(self
            .client
            .dataset(&self.dataset)
            .check_handle(&handle)
            .await
            .map_err(TopkError::from)?)
    }

    /// Poll until a handle has been processed or the timeout is reached.
    ///
    /// Throws if the handle is not processed within the configured timeout.
    #[napi]
    pub async fn wait_for_handle(&self, handle: String, config: Option<WaitConfig>) -> Result<()> {
        Ok(self
            .client
            .dataset(&self.dataset)
            .wait_for_handle(&handle, config.map(|c| c.into()))
            .await
            .map_err(TopkError::from)?)
    }

    /// List files in the dataset.
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

    /// List files in the dataset as an async iterator.
    #[napi]
    pub fn list_stream(
        &self,
        fields: Option<Vec<String>>,
        #[napi(ts_arg_type = "query.LogicalExpression | undefined")] filter: Option<
            &LogicalExpression,
        >,
    ) -> DatasetListStream {
        let (tx, rx) = mpsc::channel::<DatasetListStreamMessage>(STREAM_BUFFER_SIZE);
        let client = self.client.clone();
        let dataset = self.dataset.clone();
        let filter = filter.map(|f| f.clone().into());

        spawn_stream_task(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| format!("failed to build tokio runtime: {e}"))?;

            runtime.block_on(async move {
                let mut stream = match client.dataset(&dataset).list(fields, filter).await {
                    Ok(response) => response.into_inner(),
                    Err(error) => {
                        let _ = tx.send(Err(format!("{error}"))).await;
                        return;
                    }
                };

                while let Some(result) = stream.next().await {
                    let message = match result {
                        Ok(entry) => Ok(entry.into()),
                        Err(error) => Err(format!("stream error: {error}")),
                    };

                    if tx.send(message).await.is_err() {
                        break;
                    }
                }
            });

            Ok(())
        });

        DatasetListStream::new(rx)
    }
}
