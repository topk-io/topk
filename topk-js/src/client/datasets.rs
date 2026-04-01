use crate::data::Dataset;
use crate::error::TopkError;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;

/// Client for managing datasets.
///
/// This client provides methods to create, list, get, and delete datasets.
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

    /// Lists all datasets in the current project.
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

    /// Retrieves information about a specific dataset.
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

    /// Creates a new dataset.
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

    /// Deletes a dataset and all its data.
    ///
    /// <Warning>
    ///   This operation is irreversible and will permanently delete all data in the dataset.
    /// </Warning>
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
