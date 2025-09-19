use crate::{data::Collection, error::TopkError, schema::field_spec::FieldSpec};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;
use topk_rs::proto::v1::control::{self};

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

    /// Lists all collections in the current project.
    #[napi]
    pub async fn list(&self) -> Result<Vec<Collection>> {
        let collections = self
            .client
            .collections()
            .list()
            .await
            .map_err(TopkError::from)?;

        Ok(collections.into_iter().map(|c| c.into()).collect())
    }

    /// Creates a new collection with the specified schema.
    #[napi]
    pub async fn create(
        &self,
        name: String,
        #[napi(ts_arg_type = "Record<string, schema.FieldSpec>")] schema: HashMap<
            String,
            FieldSpec,
        >,
    ) -> Result<Collection> {
        let proto_schema: HashMap<String, control::FieldSpec> = schema
            .into_iter()
            .map(|(k, v)| (k, v.clone().into()))
            .collect();

        let collection = self
            .client
            .collections()
            .create(name, proto_schema)
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
