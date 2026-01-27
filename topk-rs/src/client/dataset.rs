use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::sync::OnceCell;
use tokio_stream::wrappers::ReceiverStream;

use crate::client::create_dataset_client;
use crate::proto::v1::ctx::file::FileId;
use crate::proto::v1::ctx::file::InputFile;
use crate::proto::v1::ctx::handle::Handle;
use crate::proto::v1::ctx::{
    upsert_message, CheckHandleRequest, DeleteRequest, GetMetadataRequest, UpdateMetadataRequest,
    UpsertMessage,
};
use crate::proto::v1::data::Value;
use crate::retry::call_with_retry;
use crate::ClientConfig;
use crate::Error;

// Buffer size for the upsert stream
const UPLOAD_BATCH_SIZE: usize = 262_144; // 256KB

// Max number of chunks in the upsert stream
const MAX_CHUNKS_IN_FLIGHT: usize = 100;

#[derive(Clone)]
pub struct DatasetClient {
    // Client config
    config: Arc<ClientConfig>,
    // Channel
    control_channel: Arc<OnceCell<tonic::transport::Channel>>,
    // Dataset name
    dataset_name: String,
}

impl DatasetClient {
    pub fn new(
        config: Arc<ClientConfig>,
        channel: Arc<OnceCell<tonic::transport::Channel>>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            config,
            control_channel: channel,
            dataset_name: name.into(),
        }
    }

    pub async fn upsert_file(
        &self,
        id: FileId,
        path: impl Into<PathBuf>,
        metadata: impl Into<HashMap<String, Value>>,
    ) -> Result<Handle, Error> {
        let client =
            create_dataset_client(&self.config, &self.dataset_name, &self.control_channel).await?;
        let path = path.into();
        let file = InputFile::from_path(path)?.is_file().await?;
        let metadata = metadata.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let metadata = metadata.clone();
            let id = id.clone().into();
            let file = file.clone();

            // Channel for the upsert stream
            let (tx, rx) = mpsc::channel(MAX_CHUNKS_IN_FLIGHT);

            // Upload task
            let upload = tokio::spawn(async move { stream_file(id, &file, metadata, tx).await });

            async move {
                let res: tonic::Response<crate::proto::v1::ctx::UpsertResponse> = client
                    .upsert(ReceiverStream::new(rx))
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })?;

                match upload.await {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => return Err(e),
                    Err(e) => {
                        return Err(Error::Internal(format!("upload task failed: {e}")));
                    }
                }

                Ok(res)
            }
        })
        .await?;

        Ok(response.into_inner().handle.into())
    }

    pub async fn delete(&self, id: FileId) -> Result<Handle, Error> {
        let client =
            create_dataset_client(&self.config, &self.dataset_name, &self.control_channel).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let id = id.clone().into();
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

        Ok(response.into_inner().handle.into())
    }

    pub async fn check_handle(&self, handle: Handle) -> Result<bool, Error> {
        let client =
            create_dataset_client(&self.config, &self.dataset_name, &self.control_channel).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let handle = handle.clone().into();
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

    pub async fn get_metadata(&self, id: FileId) -> Result<HashMap<String, Value>, Error> {
        let client =
            create_dataset_client(&self.config, &self.dataset_name, &self.control_channel).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let id = id.clone().into();

            async move {
                client
                    .get_metadata(GetMetadataRequest { id })
                    .await
                    .map_err(|e| match e.code() {
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        _ => Error::from(e),
                    })
            }
        })
        .await?;

        Ok(response.into_inner().metadata)
    }

    pub async fn update_metadata(
        &self,
        id: FileId,
        metadata: HashMap<String, Value>,
    ) -> Result<Handle, Error> {
        let client =
            create_dataset_client(&self.config, &self.dataset_name, &self.control_channel).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let id = id.clone().into();
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

        Ok(response.into_inner().handle.into())
    }
}

async fn stream_file(
    id: FileId,
    input: &InputFile,
    metadata: HashMap<String, Value>,
    tx: mpsc::Sender<UpsertMessage>,
) -> Result<(), Error> {
    let mut file = tokio::fs::File::open(&input.path).await?;

    let size = file.metadata().await?.len();

    // Send header
    tx.send(UpsertMessage {
        message: Some(upsert_message::Message::Header(upsert_message::Header {
            id: id.into(),
            kind: input.kind.into(),
            metadata,
            size,
            file_name: input.file_name.clone(),
        })),
    })
    .await
    .map_err(|e| Error::Internal(format!("failed to send header: {e}")))?;

    let mut buf = [0u8; UPLOAD_BATCH_SIZE];
    let mut seq = 0;
    loop {
        let n = file.read(&mut buf).await?;

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
