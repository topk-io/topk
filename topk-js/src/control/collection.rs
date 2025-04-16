use crate::data::document::NapiDocument;
use crate::data::query::Query;
use crate::data::value::Value;
use crate::error::TopkError;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;

use super::data_type::DataType;
use super::field_index::FieldIndexUnion;

// use super::field_spec::FieldSpec;

#[napi]
pub struct CollectionClient {
    collection: String,
    client: Arc<topk_rs::Client>,
}

#[napi(string_enum = "lowercase")]
#[derive(Debug, Clone)]
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

    // TODO: Refactor lsn to be a string
    #[napi]
    pub async fn get(
        &self,
        id: String,
        fields: Option<Vec<String>>,
        lsn: Option<i64>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<HashMap<String, Value>> {
        let document = self
            .client
            .collection(&self.collection)
            .get(
                id,
                fields.unwrap_or_default(),
                lsn.map(|l| l as u64),
                consistency.map(|c| c.into()),
            )
            .await
            .map_err(TopkError::from)?;

        Ok(document
            .fields
            .into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect())
    }

    // TODO: Refactor lsn to be a string
    #[napi]
    pub async fn count(
        &self,
        lsn: Option<i64>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<i64> {
        let query = Query::new().count();

        let docs = self
            .client
            .collection(&self.collection)
            .query(
                query.into(),
                lsn.map(|l| l as u64),
                consistency.map(|c| c.into()),
            )
            .await
            .map_err(TopkError::from)?;

        for doc in docs {
            match doc.fields.get("_count") {
                Some(value) => match value.as_u64() {
                    Some(count) => return Ok(count as i64),
                    None => {
                        return Err(napi::Error::new(
                            napi::Status::GenericFailure,
                            "Invalid _count field data type in count query response",
                        ))
                    }
                },
                None => {
                    return Err(napi::Error::new(
                        napi::Status::GenericFailure,
                        "Missing _count field in count query response",
                    ))
                }
            }
        }

        Err(napi::Error::new(
            napi::Status::GenericFailure,
            "No documents received for count query",
        ))
    }

    #[napi]
    pub async fn query(
        &self,
        #[napi(ts_arg_type = "query.Query")] query: Query,
        lsn: Option<u32>,
        consistency: Option<ConsistencyLevel>,
    ) -> Result<Vec<HashMap<String, Value>>> {
        let docs = self
            .client
            .collection(&self.collection)
            .query(
                query.into(),
                lsn.map(|l| l as u64),
                consistency.map(|c| c.into()),
            )
            .await
            .map_err(TopkError::from)?;

        Ok(docs
            .into_iter()
            .map(|d| NapiDocument::from(d).into())
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
            .map_err(TopkError::from)
            .map(|lsn| lsn as i64)?;

        Ok(result)
    }

    #[napi]
    pub async fn delete(&self, ids: Vec<String>) -> Result<i64> {
        let result = self
            .client
            .collection(&self.collection)
            .delete(ids)
            .await
            .map_err(TopkError::from)
            .map(|lsn| lsn as i64)?;

        Ok(result)
    }
}

#[napi(object)]
pub struct FieldSpec {
    #[napi(ts_type = "schema.DataType")]
    pub data_type: DataType,
    pub required: bool,
    #[napi(ts_type = "schema.FieldIndexUnion")]
    pub index: Option<FieldIndexUnion>,
}

impl From<topk_protos::v1::control::FieldSpec> for FieldSpec {
    fn from(field_spec: topk_protos::v1::control::FieldSpec) -> Self {
        Self {
            data_type: field_spec.data_type.unwrap().into(),
            required: field_spec.required,
            index: field_spec.index.map(|index| index.into()),
        }
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
