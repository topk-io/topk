use super::client_config::ClientConfig;
use super::create_collection_client;
use crate::error::Error;
use crate::error::ValidationErrorBag;
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
        let mut client = create_collection_client(&self.config, &self.control_channel).await?;

        let response = client
            .list_collections(ListCollectionsRequest {})
            .await
            .map_err(|e| match e.code() {
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collections)
    }

    pub async fn get(&self, name: impl Into<String>) -> Result<Collection, Error> {
        let mut client = create_collection_client(&self.config, &self.control_channel).await?;

        let response = client
            .get_collection(GetCollectionRequest { name: name.into() })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collection.expect("invalid proto"))
    }

    pub async fn create(
        &self,
        name: impl Into<String>,
        schema: impl Into<HashMap<String, FieldSpec>>,
    ) -> Result<Collection, Error> {
        let mut client = create_collection_client(&self.config, &self.control_channel).await?;

        let response = client
            .create_collection(CreateCollectionRequest {
                name: name.into(),
                schema: schema.into(),
            })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::AlreadyExists => Error::CollectionAlreadyExists,
                tonic::Code::InvalidArgument => {
                    if let Ok(errors) = ValidationErrorBag::try_from(e.clone()) {
                        Error::CollectionValidationError(errors)
                    } else if let Ok(errors) = ValidationErrorBag::try_from(e.clone()) {
                        Error::SchemaValidationError(errors)
                    } else {
                        Error::Unexpected(e)
                    }
                }
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collection.expect("invalid proto"))
    }

    pub async fn delete(&self, name: impl Into<String>) -> Result<(), Error> {
        let mut client = create_collection_client(&self.config, &self.control_channel).await?;

        client
            .delete_collection(DeleteCollectionRequest { name: name.into() })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                _ => Error::Unexpected(e),
            })?;

        Ok(())
    }
}
