use crate::data::document::NapiDocument;
use crate::data::value::Value;
use crate::error::TopkError;
use crate::query::query::Query;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

#[napi]
pub struct CollectionClient {
    collection: String,
    client: Arc<topk_rs::Client>,
}

#[napi(object)]
#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub lsn: Option<String>,
    pub consistency: Option<ConsistencyLevel>,
}

#[napi(string_enum = "camelCase")]
#[derive(Debug, Clone, Copy)]
pub enum ConsistencyLevel {
    Indexed,
    Strong,
}

impl From<ConsistencyLevel> for topk_protos::v1::data::ConsistencyLevel {
    fn from(consistency_level: ConsistencyLevel) -> Self {
        match consistency_level {
            ConsistencyLevel::Indexed => topk_protos::v1::data::ConsistencyLevel::Indexed,
            ConsistencyLevel::Strong => topk_protos::v1::data::ConsistencyLevel::Strong,
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
        let (lsn, consistency) = match &options {
            Some(o) => (o.lsn.clone(), o.consistency.as_ref().map(|c| (*c).into())),
            None => (None, None),
        };

        let documents = self
            .client
            .collection(&self.collection)
            .get(ids, fields, lsn, consistency)
            .await
            .map_err(TopkError::from)?;

        Ok(documents
            .into_iter()
            .map(|(id, doc)| (id, doc.into_iter().map(|(k, v)| (k, v.into())).collect()))
            .collect())
    }

    #[napi]
    pub async fn count(&self, options: Option<QueryOptions>) -> Result<i64> {
        let (lsn, consistency) = match &options {
            Some(o) => (o.lsn.clone(), o.consistency.as_ref().map(|c| (*c).into())),
            None => (None, None),
        };

        let count = self
            .client
            .collection(&self.collection)
            .count(lsn, consistency)
            .await
            .map_err(TopkError::from)?;

        Ok(count as i64)
    }

    #[napi]
    pub async fn query(
        &self,
        #[napi(ts_arg_type = "query.Query")] query: Query,
        options: Option<QueryOptions>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let (lsn, consistency) = match &options {
            Some(o) => (o.lsn.clone(), o.consistency.as_ref().map(|c| (*c).into())),
            None => (None, None),
        };

        let docs = self
            .client
            .collection(&self.collection)
            .query(query.into(), lsn, consistency)
            .await
            .map_err(TopkError::from)?;

        Ok(docs
            .into_iter()
            .map(|d| NapiDocument::from(d).into())
            .collect())
    }

    #[napi]
    pub async fn upsert(&self, docs: Vec<HashMap<String, Value>>) -> Result<String> {
        let lsn = self
            .client
            .collection(&self.collection)
            .upsert(
                docs.into_iter()
                    .map(|d| topk_protos::v1::data::Document {
                        fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
                    })
                    .collect(),
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
