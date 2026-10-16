use super::{RUNTIME, STREAM_BUFFER_SIZE};
use crate::data::{Collection, CollectionListStream, CollectionListStreamMessage};
use crate::{error::TopkError, schema::field_spec::FieldSpec};
use futures_util::StreamExt;
use napi::bindgen_prelude::*;
use napi::tokio::{self, sync::mpsc};
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;
use topk_rs::proto::v1::control::{self};

/// Options for listing collections.
#[napi(object)]
#[derive(Default)]
pub struct ListCollectionsOptions {
    /// Only list collections whose name starts with this prefix
    pub prefix: Option<String>,
}

/// Client for managing collections.
///
/// This client provides methods to create, list, get, and delete collections.
/// @internal
/// @hideconstructor
#[napi]
pub struct CollectionsClient {
    /// Reference to the topk-rs client
    client: Arc<topk_rs::Client>,
}

#[napi]
impl CollectionsClient {
    pub fn new(client: Arc<topk_rs::Client>) -> Self {
        Self { client }
    }

    /// Lists collections in the current project, ordered by name, as an async iterator.
    #[napi]
    pub fn list(&self, options: Option<ListCollectionsOptions>) -> CollectionListStream {
        let ListCollectionsOptions { prefix } = options.unwrap_or_default();
        let (tx, rx) = mpsc::channel::<CollectionListStreamMessage>(STREAM_BUFFER_SIZE);
        let client = self.client.clone();

        RUNTIME.spawn(async move {
            let mut stream = match client.collections().list(prefix).await {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = tx.send(Err(format!("{error}"))).await;
                    return;
                }
            };

            loop {
                tokio::select! {
                    _ = tx.closed() => break,
                    result = stream.next() => {
                        let Some(result) = result else { break };

                        let message = match result {
                            Ok(collection) => Ok(collection.into()),
                            Err(error) => Err(format!("stream error: {error}")),
                        };

                        if tx.send(message).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        CollectionListStream::new(rx)
    }

    /// Creates a new collection with the specified schema.
    #[napi]
    pub async fn create(
        &self,
        name: String,
        #[napi(ts_arg_type = "Record<string, SchemaFieldSpec>")] schema: HashMap<String, FieldSpec>,
    ) -> Result<Collection> {
        let proto_schema: HashMap<String, control::FieldSpec> = schema
            .into_iter()
            .map(|(k, v)| (k, v.clone().into()))
            .collect();

        let collection = self
            .client
            .collections()
            .create(name, proto_schema, None)
            .await
            .map_err(TopkError::from)?;

        Ok(collection.into())
    }

    /// Retrieves information about a specific collection.
    #[napi]
    pub async fn get(&self, name: String) -> Result<Collection> {
        let collection = self
            .client
            .collections()
            .get(&name)
            .await
            .map_err(TopkError::from)?;

        Ok(collection.into())
    }

    /// Deletes a collection and all its data.
    ///
    /// <Warning>
    ///   This operation is irreversible and will permanently delete all data in the collection.
    /// </Warning>
    #[napi]
    pub async fn delete(&self, name: String) -> Result<()> {
        self.client
            .collections()
            .delete(name)
            .await
            .map_err(TopkError::from)?;

        Ok(())
    }
}
