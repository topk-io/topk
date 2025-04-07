use crate::data::document::DocumentWrapper;
use crate::data::query::Query;
use crate::data::value::Value;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

use super::field_spec::FieldSpec;

#[napi]
pub struct CollectionClient {
    collection: String,
    client: Arc<topk_rs::Client>,
}

#[napi]
impl CollectionClient {
    pub fn new(client: Arc<topk_rs::Client>, collection: String) -> Self {
        Self { client, collection }
    }

    #[napi]
    pub async fn query(
        &self,
        query: Query,
        lsn: Option<u32>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let docs = self
            .client
            .collection(&self.collection)
            .query(query.into(), lsn.map(|l| l as u64), None)
            .await
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

        Ok(docs
            .into_iter()
            .map(|d| DocumentWrapper::from(d).into())
            .collect())
    }

    #[napi]
    pub async fn upsert(&self, docs: Vec<HashMap<String, Value>>) -> Result<i64> {
        let result = self
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
            .map_err(|e| {
                napi::Error::new(
                    napi::Status::GenericFailure,
                    format!("upsert failed: {:?}", e),
                )
            })
            .map(|lsn| lsn as i64);

        result
    }

    #[napi]
    pub async fn delete(&self, ids: Vec<String>) -> Result<i64> {
        let result = self
            .client
            .collection(&self.collection)
            .delete(ids)
            .await
            .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

        Ok(result as i64)
    }
}

#[napi(object)]
pub struct Collection {
    pub name: String,
    pub org_id: String,
    pub project_id: String,
    pub schema: HashMap<String, FieldSpec>,
    pub region: String,
}

impl From<topk_protos::v1::control::Collection> for Collection {
    fn from(collection: topk_protos::v1::control::Collection) -> Self {
        Self {
            name: collection.name,
            org_id: collection.org_id,
            project_id: collection.project_id,
            schema: collection
                .schema
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            region: collection.region,
        }
    }
}
