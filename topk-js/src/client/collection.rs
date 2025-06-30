use crate::data::Document;
use crate::data::Value;
use crate::error::TopkError;
use crate::query::query::Query;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

#[napi]
pub struct CollectionClient {
    /// Name of the collection
    collection: String,
    /// Reference to the topk-rs client
    client: Arc<topk_rs::Client>,
}

#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Last sequence number to query at
    pub lsn: Option<String>,
    /// Consistency level
    pub consistency: Option<ConsistencyLevel>,
}

#[napi(string_enum = "camelCase")]
#[derive(Debug, Clone, Copy)]
pub enum ConsistencyLevel {
    Indexed,
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

    #[napi]
    pub async fn get(
        &self,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
        options: Option<QueryOptions>,
    ) -> Result<HashMap<String, HashMap<String, Value>>> {
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

    #[napi]
    pub async fn query(
        &self,
        #[napi(ts_arg_type = "query.Query")] query: &Query,
        options: Option<QueryOptions>,
    ) -> Result<Vec<HashMap<String, Value>>> {
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

        Ok(docs.into_iter().map(|d| Document::from(d).into()).collect())
    }

    #[napi]
    pub async fn upsert(&self, docs: Vec<HashMap<String, Value>>) -> Result<String> {
        let lsn = self
            .client
            .collection(&self.collection)
            .upsert(
                docs.into_iter()
                    .map(|d| Document::new(d).into())
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(TopkError::from)?;

        Ok(lsn)
    }

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
