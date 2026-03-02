use std::sync::Arc;

use futures_util::TryFutureExt;
use tokio::sync::OnceCell;

use super::config::ClientConfig;
use super::create_datasets_client;
use super::retry::call_with_retry;
use super::Response;
use crate::error::Error;
use crate::proto::v1::control::{
    CreateDatasetRequest, CreateDatasetResponse, DeleteDatasetRequest, DeleteDatasetResponse,
    GetDatasetRequest, GetDatasetResponse, ListDatasetsRequest, ListDatasetsResponse,
};

pub struct DatasetsClient {
    // Client config
    config: Arc<ClientConfig>,
    // Channel
    control_channel: OnceCell<tonic::transport::Channel>,
}

impl DatasetsClient {
    pub fn new(config: &ClientConfig, channel: &OnceCell<tonic::transport::Channel>) -> Self {
        Self {
            config: Arc::new(config.clone()),
            control_channel: channel.clone(),
        }
    }

    pub async fn list(&self) -> Result<Response<ListDatasetsResponse>, Error> {
        let client = create_datasets_client(&self.config, &self.control_channel).await?;

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

        Ok(response.into())
    }

    pub async fn get(
        &self,
        name: impl Into<String>,
    ) -> Result<Response<GetDatasetResponse>, Error> {
        let client = create_datasets_client(&self.config, &self.control_channel).await?;
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

        Ok(response.into())
    }

    pub async fn create(
        &self,
        name: impl Into<String>,
    ) -> Result<Response<CreateDatasetResponse>, Error> {
        let client = create_datasets_client(&self.config, &self.control_channel).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config(), || {
            let mut client = client.clone();
            let name = name.clone();

            async move {
                client
                    .create_dataset(CreateDatasetRequest { name })
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

        Ok(response.into())
    }

    pub async fn delete(
        &self,
        name: impl Into<String>,
    ) -> Result<Response<DeleteDatasetResponse>, Error> {
        let client = create_datasets_client(&self.config, &self.control_channel).await?;
        let name = name.into();

        let response = call_with_retry(&self.config.retry_config(), || {
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

        Ok(response.into())
    }
}
