use crate::data::NativeValue;
use crate::data::Value;
use crate::error::TopkError;
use crate::query::query::Query;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

/// Client for interacting with a specific collection.
///
/// This client provides methods to perform operations on a specific collection,
/// including querying, upserting, and deleting documents.
/// @internal
/// @hideconstructor
#[napi]
pub struct CollectionClient {
    /// Name of the collection
    collection: String,
    /// Reference to the topk-rs client
    client: Arc<topk_rs::Client>,
}

/// Options for query operations.
///
/// These options control the behavior of query operations, including consistency
/// guarantees and sequence number constraints.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Last sequence number to query at (for consistency)
    pub lsn: Option<String>,
    /// Consistency level for the query
    pub consistency: Option<ConsistencyLevel>,
}

/// Consistency levels for query operations.
///
/// - `Indexed`: Query returns results as soon as they are indexed (faster, eventual consistency)
/// - `Strong`: Query waits for all replicas to be consistent (slower, strong consistency)
#[napi(string_enum = "camelCase")]
#[derive(Debug, Clone, Copy)]
pub enum ConsistencyLevel {
    /// Indexed consistency - faster, eventual consistency
    Indexed,
    /// Strong consistency - slower, waits for all replicas
    Strong,
}

impl From<ConsistencyLevel> for topk_rs::proto::v1::data::ConsistencyLevel {
    fn from(consistency_level: ConsistencyLevel) -> Self {
        match consistency_level {
            ConsistencyLevel::Indexed => topk_rs::proto::v1::data::ConsistencyLevel::Indexed,
            ConsistencyLevel::Strong => topk_rs::proto::v1::data::ConsistencyLevel::Strong,
        }
    }
}

#[napi]
impl CollectionClient {
    pub fn new(client: Arc<topk_rs::Client>, collection: String) -> Self {
        Self { client, collection }
    }

    /// Retrieves documents by their IDs.
    #[napi(ts_return_type = "Promise<Record<string, Record<string, any>>>")]
    pub async fn get(
        &self,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
        options: Option<QueryOptions>,
    ) -> Result<HashMap<String, HashMap<String, NativeValue>>> {
        let options = options.unwrap_or_default();

        let documents = self
            .client
            .collection(&self.collection)
            .get(
                ids,
                fields,
                options.lsn,
                options.consistency.map(|c| c.into()),
            )
            .await
            .map_err(TopkError::from)?;

        Ok(documents
            .into_iter()
            .map(|(id, doc)| (id, doc.into_iter().map(|(k, v)| (k, v.into())).collect()))
            .collect())
    }

    /// Counts the number of documents in the collection.
    #[napi]
    pub async fn count(&self, options: Option<QueryOptions>) -> Result<u32> {
        let options = options.unwrap_or_default();

        let count = self
            .client
            .collection(&self.collection)
            .count(options.lsn, options.consistency.map(|c| c.into()))
            .await
            .map_err(TopkError::from)?;

        Ok(count as u32)
    }

    /// Executes a query against the collection.
    #[napi(ts_return_type = "Promise<Array<Record<string, any>>>")]
    pub async fn query(
        &self,
        #[napi(ts_arg_type = "query.Query")] query: &Query,
        options: Option<QueryOptions>,
    ) -> Result<Vec<HashMap<String, NativeValue>>> {
        let options = options.unwrap_or_default();

        let docs = self
            .client
            .collection(&self.collection)
            .query(
                query.clone().into(),
                options.lsn,
                options.consistency.map(|c| c.into()),
            )
            .await
            .map_err(TopkError::from)?;

        Ok(docs
            .into_iter()
            .map(|d| d.fields.into_iter().map(|(k, v)| (k, v.into())).collect())
            .collect())
    }

    /// Inserts or updates documents in the collection.
    #[napi]
    pub async fn upsert(&self, docs: Vec<HashMap<String, Value>>) -> Result<String> {
        let documents = docs
            .into_iter()
            .map(|d| topk_rs::proto::v1::data::Document {
                fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })
            .collect();

        let lsn = self
            .client
            .collection(&self.collection)
            .upsert(documents)
            .await
            .map_err(TopkError::from)?;

        Ok(lsn)
    }

    /// Deletes documents from the collection by their IDs.
    #[napi]
    pub async fn delete(&self, ids: Vec<String>) -> Result<String> {
        let lsn = self
            .client
            .collection(&self.collection)
            .delete(ids)
            .await
            .map_err(TopkError::from)?;

        Ok(lsn)
    }
}
