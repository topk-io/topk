use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::data::Dataset;
use crate::error::TopkError;

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
        let datasets = self
            .client
            .datasets()
            .list()
            .await
            .map_err(TopkError::from)?;

        Ok(datasets.into_iter().map(|d| d.into()).collect())
    }

    /// Get information about a specific dataset.
    #[napi]
    pub async fn get(&self, name: String) -> Result<Dataset> {
        let dataset = self
            .client
            .datasets()
            .get(&name)
            .await
            .map_err(TopkError::from)?;

        Ok(dataset.into())
    }

    /// Create a new dataset.
    #[napi]
    pub async fn create(&self, name: String) -> Result<Dataset> {
        let dataset = self
            .client
            .datasets()
            .create(&name, None)
            .await
            .map_err(TopkError::from)?;

        Ok(dataset.into())
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
