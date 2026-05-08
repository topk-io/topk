use std::sync::Arc;

use futures_util::TryFutureExt;
use tokio::sync::OnceCell;
use tonic::transport::Channel;

use super::config::ClientConfig;
use super::retry::call_with_retry;
use crate::create_client;
use crate::error::Error;
use crate::proto::v1::control::dataset_service_client::DatasetServiceClient;
use crate::proto::v1::control::{
    CreateDatasetRequest, Dataset, DeleteDatasetRequest, GetDatasetRequest, ListDatasetsRequest,
    UpdateDatasetRequest,
};

pub struct DatasetsClient {
    // Client config
    config: ClientConfig,
    // Channel
    channel: Arc<OnceCell<Channel>>,
}

impl DatasetsClient {
    pub fn new(config: ClientConfig, channel: Arc<OnceCell<Channel>>) -> Self {
        Self { config, channel }
    }

    pub async fn list(&self) -> Result<Vec<Dataset>, Error> {
        let client = create_client!(DatasetServiceClient, self.channel, self.config).await?;

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();

            async move {
                client
                    .list_datasets(ListDatasetsRequest {})
                    .map_err(Error::from)
                    .await
            }
        })
        .await?;

        Ok(response.into_inner().datasets)
    }

    pub async fn get(&self, name: impl Into<String>) -> Result<Dataset, Error> {
        let client = create_client!(DatasetServiceClient, self.channel, self.config).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();

            async move {
                client
                    .get_dataset(GetDatasetRequest { name })
                    .map_err(|e| match e.code() {
                        // Dataset not found
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        // Delegate other errors
                        _ => Error::from(e),
                    })
                    .await
            }
        })
        .await?;

        let dataset = response.into_inner().dataset.ok_or(Error::InvalidProto)?;
        Ok(dataset)
    }

    pub async fn create(
        &self,
        name: impl Into<String>,
        region: Option<String>,
    ) -> Result<Dataset, Error> {
        let client = create_client!(DatasetServiceClient, self.channel, self.config).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();
            let region = region.clone();

            async move {
                client
                    .create_dataset(CreateDatasetRequest { name, region })
                    .await
                    .map_err(|e| match e.code() {
                        // Dataset already exists
                        tonic::Code::AlreadyExists => Error::DatasetAlreadyExists,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        let dataset = response.into_inner().dataset.ok_or(Error::InvalidProto)?;
        Ok(dataset)
    }

    pub async fn update(
        &self,
        name: impl Into<String>,
        description: Option<String>,
    ) -> Result<Dataset, Error> {
        let client = create_client!(DatasetServiceClient, self.channel, self.config).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();
            let description = description.clone();

            async move {
                client
                    .update_dataset(UpdateDatasetRequest { name, description })
                    .await
                    .map_err(|e| match e.code() {
                        // Dataset not found
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        let dataset = response.into_inner().dataset.ok_or(Error::InvalidProto)?;
        Ok(dataset)
    }

    pub async fn delete(&self, name: impl Into<String>) -> Result<(), Error> {
        let client = create_client!(DatasetServiceClient, self.channel, self.config).await?;
        let name = name.into();

        call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();

            async move {
                client
                    .delete_dataset(DeleteDatasetRequest { name })
                    .await
                    .map_err(|e| match e.code() {
                        // Dataset not found
                        tonic::Code::NotFound => Error::DatasetNotFound,
                        // Delegate other errors
                        _ => e.into(),
                    })
            }
        })
        .await?;

        Ok(())
    }
}
