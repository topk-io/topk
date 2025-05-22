use super::config::ClientConfig;
use super::create_collection_client;
use super::retry::call_with_retry;
use crate::error::Error;
use futures_util::TryFutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;
use topk_protos::v1::control::{
    Collection, CreateCollectionRequest, DeleteCollectionRequest, ListCollectionsRequest,
};
use topk_protos::v1::control::{FieldSpec, GetCollectionRequest};

pub struct CollectionsClient {
    // Client config
    config: Arc<ClientConfig>,
    // Channel
    control_channel: OnceCell<tonic::transport::Channel>,
}

impl CollectionsClient {
    pub fn new(config: &ClientConfig, channel: &OnceCell<tonic::transport::Channel>) -> Self {
        Self {
            config: Arc::new(config.clone()),
            control_channel: channel.clone(),
        }
    }

    pub async fn list(&self) -> Result<Vec<Collection>, Error> {
        let client = create_collection_client(&self.config, &self.control_channel).await?;

        let response = call_with_retry(&self.config.retry_config, || {
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
        let client = create_collection_client(&self.config, &self.control_channel).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config, || {
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

        Ok(response.into_inner().collection.expect("invalid proto"))
    }

    pub async fn create(
        &self,
        name: impl Into<String>,
        schema: impl Into<HashMap<String, FieldSpec>>,
    ) -> Result<Collection, Error> {
        let client = create_collection_client(&self.config, &self.control_channel).await?;
        let name = name.into();
        let schema = schema.into();

        let response = call_with_retry(&self.config.retry_config, || {
            let mut client = client.clone();
            let name = name.clone();
            let schema = schema.clone();

            async move {
                client
                    .create_collection(CreateCollectionRequest { name, schema })
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

        Ok(response.into_inner().collection.expect("invalid proto"))
    }

    pub async fn delete(&self, name: impl Into<String>) -> Result<(), Error> {
        let client = create_collection_client(&self.config, &self.control_channel).await?;
        let name = name.into();

        let _ = call_with_retry(&self.config.retry_config, || {
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
