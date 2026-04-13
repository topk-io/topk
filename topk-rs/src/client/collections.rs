use std::collections::HashMap;
use std::sync::Arc;

use futures_util::TryFutureExt;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

use super::config::ClientConfig;
use super::retry::call_with_retry;
use crate::create_client;
use crate::error::Error;
use crate::proto::v1::control::collection_service_client::CollectionServiceClient;
use crate::proto::v1::control::{
    Collection, CreateCollectionRequest, DeleteCollectionRequest, ListCollectionsRequest,
};
use crate::proto::v1::control::{FieldSpec, GetCollectionRequest};

pub struct CollectionsClient {
    // Client config
    config: ClientConfig,
    // Channel
    channel: Arc<OnceCell<Channel>>,
}

impl CollectionsClient {
    pub fn new(config: ClientConfig, channel: Arc<OnceCell<Channel>>) -> Self {
        Self { config, channel }
    }

    pub async fn list(&self) -> Result<Vec<Collection>, Error> {
        let client = create_client!(CollectionServiceClient, self.channel, self.config).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();

            async move {
                client
                    .list_collections(ListCollectionsRequest {})
                    .map_err(Error::from)
                    .await
            }
        })
        .await?;

        Ok(response.into_inner().collections)
    }

    pub async fn get(&self, name: impl Into<String>) -> Result<Collection, Error> {
        let client = create_client!(CollectionServiceClient, self.channel, self.config).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();

            async move {
                client
                    .get_collection(GetCollectionRequest { name })
                    .map_err(|e| match e.code() {
                        // Collection not found
                        tonic::Code::NotFound => Error::CollectionNotFound,
                        // Delegate other errors
                        _ => Error::from(e),
                    })
                    .await
            }
        })
        .await?;

        Ok(response
            .into_inner()
            .collection
            .expect("Invalid collection proto"))
    }

    pub async fn create(
        &self,
        name: impl Into<String>,
        schema: impl Into<HashMap<String, FieldSpec>>,
    ) -> Result<Collection, Error> {
        let client = create_client!(CollectionServiceClient, self.channel, self.config).await?;
        let name = name.into();
        let schema = schema.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();
            let schema = schema.clone();

            async move {
                client
                    .create_collection(CreateCollectionRequest {
                        name,
                        schema,
                        region: None,
                    })
                    .await
                    .map_err(|e| match e.code() {
                        // Collection already exists
                        tonic::Code::AlreadyExists => Error::CollectionAlreadyExists,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        Ok(response
            .into_inner()
            .collection
            .expect("Invalid collection proto"))
    }

    pub async fn delete(&self, name: impl Into<String>) -> Result<(), Error> {
        let client = create_client!(CollectionServiceClient, self.channel, self.config).await?;
        let name = name.into();

        let _ = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();

            async move {
                client
                    .delete_collection(DeleteCollectionRequest { name })
                    .await
                    .map_err(|e| match e.code() {
                        // Collection not found
                        tonic::Code::NotFound => Error::CollectionNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        Ok(())
    }
}
