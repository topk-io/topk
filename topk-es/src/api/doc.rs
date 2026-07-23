use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use topk_rs::json::Value;
use topk_rs::proto::v1::data::Value as TopkValue;

use super::DocId;
use super::IndexName;
use crate::Error;

#[derive(Clone, Serialize)]
#[serde(transparent)]
pub struct Source(pub Value);

#[derive(Serialize)]
pub struct DocItem {
    #[serde(rename = "_index")]
    pub index: IndexName,
    #[serde(rename = "_id")]
    pub id: DocId,
    pub found: bool,
    // Kibana reads these to drive conditional updates. Constant (we don't track sequence numbers);
    // present only for a found doc.
    #[serde(rename = "_version", skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(rename = "_seq_no", skip_serializing_if = "Option::is_none")]
    pub seq_no: Option<u64>,
    #[serde(rename = "_primary_term", skip_serializing_if = "Option::is_none")]
    pub primary_term: Option<u64>,
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

#[derive(Deserialize)]
#[serde(try_from = "HashMap<String, serde_json::Value>")]
pub struct DocBody(HashMap<String, Value>);

impl TryFrom<HashMap<String, serde_json::Value>> for DocBody {
    type Error = Error;

    fn try_from(doc: HashMap<String, serde_json::Value>) -> Result<Self, Self::Error> {
        let mut fields = HashMap::with_capacity(doc.len());
        for (key, value) in doc {
            if key == "_id" {
                return Err(Error::BadRequest(
                    "\"_id\" is a metadata field and cannot be set inside the document body".into(),
                ));
            }
            let value = TopkValue::try_from(value)
                .map(Value::from)
                .map_err(|e| Error::BadRequest(e.to_string()))?;
            fields.insert(key, value);
        }
        Ok(Self(fields))
    }
}

impl DocBody {
    pub fn into_fields(self, id: &DocId) -> HashMap<String, Value> {
        let mut doc = self.0;
        doc.insert("_id".to_string(), id.to_string().into());
        doc
    }
}

pub struct WriteDoc {
    pub id: DocId,
    pub body: DocBody,
}

impl WriteDoc {
    pub fn new(id: DocId, body: DocBody) -> Self {
        Self { id, body }
    }

    pub fn into_fields(self) -> HashMap<String, Value> {
        self.body.into_fields(&self.id)
    }
}
