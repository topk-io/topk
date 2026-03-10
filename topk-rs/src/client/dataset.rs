use std::collections::HashMap;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_stream::wrappers::ReceiverStream;

use tokio::sync::OnceCell;
use tonic::transport::Channel;
use tonic::Streaming;

use crate::client::Response;
use crate::proto::v1::ctx::dataset_read_service_client::DatasetReadServiceClient;
use crate::proto::v1::ctx::dataset_write_service_client::DatasetWriteServiceClient;
use crate::proto::v1::ctx::doc::DocId;
use crate::proto::v1::ctx::file::{InputFile, InputSource};
use crate::proto::v1::ctx::ListEntry;
use crate::proto::v1::ctx::ListRequest;
use crate::proto::v1::ctx::{
    upsert_message, CheckHandleRequest, DeleteRequest, DeleteResponse,
    GetMetadataRequest, GetMetadataResponse, UpdateMetadataRequest, UpdateMetadataResponse,
    UpsertMessage, UpsertResponse,
};
use crate::proto::v1::data::LogicalExpr;
use crate::proto::v1::data::Value;
use crate::retry::call_with_retry;
use crate::Error;
use crate::{create_client, ClientConfig};

/// Configuration for polling when waiting for a handle to be processed.
#[derive(Debug, Clone)]
pub struct WaitConfig {
    /// How often to poll for the handle status.
    pub frequency: Duration,
    /// Maximum time to wait before returning a timeout error.
    pub timeout: Duration,
}

impl Default for WaitConfig {
    fn default() -> Self {
        Self {
            frequency: Duration::from_secs(5),
            timeout: Duration::from_secs(300),
        }
    }
}

// Buffer size for the upsert stream
const UPLOAD_BATCH_SIZE: usize = 262_144; // 256KB

// Max number of chunks in the upsert stream
const MAX_CHUNKS_IN_FLIGHT: usize = 100;

#[derive(Clone)]
pub struct DatasetClient {
    // Client config
    config: ClientConfig,
    // Read channel
    read: Arc<OnceCell<Channel>>,
    // Write channel
    write: Arc<OnceCell<Channel>>,
}

impl DatasetClient {
    pub fn new(
        config: ClientConfig,
        read: Arc<OnceCell<Channel>>,
        write: Arc<OnceCell<Channel>>,
    ) -> Self {
        Self {
            config,
            read,
            write,
        }
    }

    pub async fn list(
        &self,
        fields: Option<Vec<String>>,
        filter: Option<LogicalExpr>,
    ) -> Result<Response<Streaming<ListEntry>>, Error> {
        let client = create_client!(DatasetReadServiceClient, self.read, self.config).await?;
        let fields = fields.unwrap_or_default();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let filter = filter.clone();
            let fields = fields.clone();
            async move {
                client
                    .list(ListRequest { fields, filter })
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })
            }
        })
        .await?;

        Ok(response.into())
    }

    pub async fn upsert_file(
        &self,
        doc_id: impl Into<DocId>,
        input: impl Into<InputFile>,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<Value>)>,
    ) -> Result<Response<UpsertResponse>, Error> {
        let client = create_client!(DatasetWriteServiceClient, self.write, self.config).await?;
        let file = input.into();
        let metadata: HashMap<String, Value> = metadata
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        let doc_id = doc_id.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let metadata = metadata.clone();
            let id = doc_id.clone();
            let file = file.clone();

            // Channel for the upsert stream
            let (tx, rx) = mpsc::channel(MAX_CHUNKS_IN_FLIGHT);

            // Upload task
            let upload = tokio::spawn(async move { stream_file(id, &file, metadata, tx).await });

            async move {
                let res =
                    client
                        .upsert(ReceiverStream::new(rx))
                        .await
                        .map_err(|e| match e.code() {
                            tonic::Code::NotFound => Error::DatasetNotFound,
                            _ => Error::from(e),
                        });

                // Abort the upload task if upsert failed early
                let res = match res {
                    Ok(res) => res,
                    Err(e) => {
                        upload.abort();
                        return Err(e);
                    }
                };

                match upload.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => return Err(e),
                    Err(e) if e.is_cancelled() => {
                        return Err(Error::Internal("upload task was cancelled".to_string()));
                    }
                    Err(e) => {
                        return Err(Error::Internal(format!("upload task failed: {e}")));
                    }
                }

                Ok(res)
            }
        })
        .await?;

        Ok(response.into())
    }

    pub async fn delete(
        &self,
        doc_id: impl Into<DocId>,
    ) -> Result<Response<DeleteResponse>, Error> {
        let client = create_client!(DatasetWriteServiceClient, self.write, self.config).await?;

        let doc_id = doc_id.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let id = doc_id.clone().into();

            async move {
                client
                    .delete(DeleteRequest { id })
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })
            }
        })
        .await?;

        Ok(response.into())
    }

    /// Checks if a handle has been processed (single shot).
    pub async fn check_handle(&self, handle: &str) -> Result<bool, Error> {
        let client = create_client!(DatasetWriteServiceClient, self.write, self.config).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let handle = handle.to_string();
            async move {
                client
                    .check_handle(CheckHandleRequest { handle })
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })
            }
        })
        .await?;

        Ok(response.into_inner().processed)
    }

    /// Polls until a handle has been processed or the timeout is reached.
    ///
    /// # Errors
    ///
    /// Returns `Error::RetryTimeout` if the handle is not processed within the configured timeout.
    pub async fn wait_for_handle(
        &self,
        handle: &str,
        config: Option<WaitConfig>,
    ) -> Result<(), Error> {
        let config = config.unwrap_or_default();
        let start = Instant::now();
        loop {
            if start.elapsed() > config.timeout {
                return Err(Error::RetryTimeout);
            }

            if self.check_handle(handle).await? {
                return Ok(());
            }

            tokio::time::sleep(config.frequency).await;
        }
    }

    pub async fn get_metadata(
        &self,
        ids: impl IntoIterator<Item = impl Into<String>>,
        fields: Option<Vec<String>>,
    ) -> Result<Response<GetMetadataResponse>, Error> {
        let client = create_client!(DatasetReadServiceClient, self.read, self.config).await?;
        let ids = ids.into_iter().map(|id| id.into()).collect::<Vec<_>>();
        let fields = fields.unwrap_or_default();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let ids = ids.clone();
            let fields = fields.clone();

            async move {
                client
                    .get_metadata(GetMetadataRequest { ids, fields })
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })
            }
        })
        .await?;

        Ok(response.into())
    }

    pub async fn update_metadata(
        &self,
        doc_id: impl Into<DocId>,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<Value>)>,
    ) -> Result<Response<UpdateMetadataResponse>, Error> {
        let client = create_client!(DatasetWriteServiceClient, self.write, self.config).await?;

        let doc_id = doc_id.into();
        let metadata: HashMap<String, Value> = metadata
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let id = doc_id.clone().into();
            let metadata = metadata.clone();

            async move {
                client
                    .update_metadata(UpdateMetadataRequest { id, metadata })
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })
            }
        })
        .await?;

        Ok(response.into())
    }
}

async fn stream_file(
    id: impl Into<DocId>,
    input: &InputFile,
    metadata: HashMap<String, Value>,
    tx: mpsc::Sender<UpsertMessage>,
) -> Result<(), Error> {
    let id = id.into();

    let (mut reader, size) = match &input.source {
        InputSource::Path(path) => {
            let file = tokio::fs::File::open(path).await?;
            let size = file.metadata().await?.len();
            (Box::pin(file) as Pin<Box<dyn AsyncRead + Send>>, size)
        }
        InputSource::Bytes(data) => {
            let size = data.len() as u64;
            let data = Box::pin(Cursor::new(data)) as Pin<Box<dyn AsyncRead + Send>>;
            (data, size)
        }
    };

    // Send header
    tx.send(UpsertMessage {
        message: Some(upsert_message::Message::Header(upsert_message::Header {
            id: id.into(),
            mime_type: input.mime_type.clone(),
            metadata,
            size,
            name: input.file_name.clone(),
        })),
    })
    .await
    .map_err(|e| Error::Internal(format!("failed to send header: {e}")))?;

    let mut buf = [0u8; UPLOAD_BATCH_SIZE];
    let mut seq = 0;
    loop {
        let n = reader.read(&mut buf).await?;

        // No more data to read
        if n == 0 {
            break;
        }

        tx.send(UpsertMessage {
            message: Some(upsert_message::Message::BodyChunk(
                upsert_message::BodyChunk {
                    data: Bytes::copy_from_slice(&buf[..n]).into(),
                    seq,
                },
            )),
        })
        .await
        .map_err(|e| Error::Internal(format!("failed to send chunk: {e}")))?;

        seq += 1;
    }

    Ok(())
}
