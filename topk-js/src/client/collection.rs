use std::collections::HashMap;
use std::sync::Arc;

use futures_util::StreamExt;
use napi::bindgen_prelude::*;
use napi::tokio::{self, sync::mpsc};
use napi_derive::napi;

use super::partition::{PartitionListStream, PartitionListStreamMessage};
use super::{RUNTIME, STREAM_BUFFER_SIZE};
use crate::data::NativeValue;
use crate::data::Value;
use crate::error::TopkError;
use crate::expr::delete::DeleteExpression;
use crate::query::query::Query;

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
    /// Optional partition name
    partition: Option<String>,
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

impl CollectionClient {
    pub fn new(client: Arc<topk_rs::Client>, collection: String, partition: Option<String>) -> Self {
        Self {
            client,
            collection,
            partition,
        }
    }

    /// Get partition-aware collection client
    fn collection(&self) -> topk_rs::CollectionClient {
        let c = self.client.collection(&self.collection);
        match &self.partition {
            Some(p) => c.partition(p.clone()),
            None => c,
        }
    }
}

#[napi]
impl CollectionClient {
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
            .collection()
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
            .collection()
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
            .collection()
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
            .collection()
            .upsert(documents)
            .await
            .map_err(TopkError::from)?;

        Ok(lsn)
    }

    /// Updates documents in the collection.
    ///
    /// Existing documents will be merged with the provided fields.
    /// Missing documents will be ignored.
    ///
    /// @returns The `LSN` at which the update was applied.
    /// If no updates were applied, this will be empty.
    #[napi]
    pub async fn update(
        &self,
        docs: Vec<HashMap<String, Value>>,
        fail_on_missing: Option<bool>,
    ) -> Result<String> {
        let documents = docs
            .into_iter()
            .map(|d| topk_rs::proto::v1::data::Document {
                fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
            })
            .collect();

        let lsn = self
            .collection()
            .update(documents, fail_on_missing.unwrap_or(false))
            .await
            .map_err(TopkError::from)?;

        Ok(lsn)
    }

    /// Deletes documents from the collection by their IDs or using a filter expression.
    ///
    /// Example:
    /// Delete documents by their IDs:
    /// ```javascript
    /// await client.collection("books").delete(["id_1", "id_2"])
    /// ```
    ///
    /// Delete documents by a filter expression:
    /// ```javascript
    /// import { field } from "topk-js/query";
    ///
    /// await client.collection("books").delete(field("published_year").gt(1997))
    /// ```
    #[napi]
    pub async fn delete(
        &self,
        #[napi(ts_arg_type = "Array<string> | query.LogicalExpression")] expr: DeleteExpression,
    ) -> Result<String> {
        let lsn = self
            .collection()
            .delete(expr)
            .await
            .map_err(TopkError::from)?;

        Ok(lsn)
    }

    /// List partitions in the collection as an async iterator.
    #[napi]
    pub fn list_partitions(&self, prefix: Option<String>) -> PartitionListStream {
        let (tx, rx) = mpsc::channel::<PartitionListStreamMessage>(STREAM_BUFFER_SIZE);
        let client = self.client.clone();
        let collection = self.collection.clone();

        RUNTIME.spawn(async move {
            let mut stream = match client.collection(&collection).list_partitions(prefix).await {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = tx.send(Err(format!("{error}"))).await;
                    return;
                }
            };

            loop {
                tokio::select! {
                    _ = tx.closed() => break,
                    result = stream.next() => {
                        let Some(result) = result else { break };

                        let message = match result {
                            Ok(partition) => Ok(partition.into()),
                            Err(error) => Err(format!("stream error: {error}")),
                        };

                        if tx.send(message).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        PartitionListStream::new(rx)
    }

    /// Delete a partition and all documents within it.
    #[napi]
    pub async fn delete_partition(&self, name: String) -> Result<()> {
        self.client
            .collection(&self.collection)
            .delete_partition(name)
            .await
            .map_err(TopkError::from)?;
        Ok(())
    }
}
