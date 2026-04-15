use crate::data::Dataset;
use crate::error::TopkError;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;

/// Client for managing datasets.
/// @internal
/// @hideconstructor
#[napi]
pub struct DatasetsClient {
    client: Arc<topk_rs::Client>,
}

#[napi]
impl DatasetsClient {
    pub fn new(client: Arc<topk_rs::Client>) -> Self {
        Self { client }
    }

    /// List all datasets.
    #[napi]
    pub async fn list(&self) -> Result<Vec<Dataset>> {
        let response = self
            .client
            .datasets()
            .list()
            .await
            .map_err(TopkError::from)?;

        Ok(response
            .into_inner()
            .datasets
            .into_iter()
            .map(|d| d.into())
            .collect())
    }

    /// Get information about a specific dataset.
    #[napi]
    pub async fn get(&self, name: String) -> Result<Dataset> {
        let response = self
            .client
            .datasets()
            .get(&name)
            .await
            .map_err(TopkError::from)?;

        response
            .into_inner()
            .dataset
            .map(|d| d.into())
            .ok_or_else(|| napi::Error::from_reason("dataset not found in response"))
    }

    /// Create a new dataset.
    #[napi]
    pub async fn create(&self, name: String) -> Result<Dataset> {
        let response = self
            .client
            .datasets()
            .create(&name)
            .await
            .map_err(TopkError::from)?;

        response
            .into_inner()
            .dataset
            .map(|d| d.into())
            .ok_or_else(|| napi::Error::from_reason("dataset not found in response"))
    }

    /// Delete a dataset.
    #[napi]
    pub async fn delete(&self, name: String) -> Result<()> {
        self.client
            .datasets()
            .delete(name)
            .await
            .map_err(TopkError::from)?;

        Ok(())
    }
}
