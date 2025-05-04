use super::{Channel, ClientConfig};
use crate::error::Error;
use crate::error::ValidationErrorBag;
use std::collections::HashMap;
use topk_protos::utils::{CollectionClient as ProtoCollectionClient, CollectionClientWithHeaders};
use topk_protos::v1::control::{
    Collection, CreateCollectionRequest, DeleteCollectionRequest, ListCollectionsRequest,
};
use topk_protos::v1::control::{FieldSpec, GetCollectionRequest};

pub struct CollectionsClient {
    config: ClientConfig,
    channel: Channel,
}

impl CollectionsClient {
    pub fn new(config: ClientConfig, channel: Channel) -> Self {
        Self { config, channel }
    }

    pub async fn list(&self) -> Result<Vec<Collection>, Error> {
        let response = self
            .client()
            .await?
            .list_collections(ListCollectionsRequest {})
            .await
            .map_err(|e| match e.code() {
                _ => Error::Unexpected(e),
            })?;

        Ok(response.into_inner().collections)
    }

    pub async fn get(&self, name: impl Into<String>) -> Result<Collection, Error> {
        let response = self
            .client()
            .await?
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
        let response = self
            .client()
            .await?
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
        self.client()
            .await?
            .delete_collection(DeleteCollectionRequest { name: name.into() })
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => Error::CollectionNotFound,
                _ => Error::Unexpected(e),
            })?;

        Ok(())
    }

    //

    async fn client(&self) -> Result<CollectionClientWithHeaders, Error> {
        Ok(ProtoCollectionClient::with_headers(
            self.channel.get().await?,
            self.config.headers(),
        ))
    }
}
