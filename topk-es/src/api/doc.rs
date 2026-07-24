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
        for (key, mut value) in doc {
            if key == "_id" {
                return Err(Error::BadRequest(
                    "\"_id\" is a metadata field and cannot be set inside the document body".into(),
                ));
            }
            stringify_object_arrays(&mut value);
            let value = TopkValue::try_from(value)
                .map(Value::from)
                .map_err(|e| Error::BadRequest(e.to_string()))?;
            fields.insert(key, value);
        }
        Ok(Self(fields))
    }
}

// ES lets any field — including an `object`-mapped one, unlike explicit `nested` — hold an
// implicit array of values. TopK's generic JSON→Value conversion only accepts arrays of numbers,
// strings, or numeric arrays, with no column for an array of structs. Pre-stringify any such array
// to a JSON blob here (same fallback `engine::doc::coerce` uses for a schema-mapped `nested`/
// `object` field) so it converts as a plain string instead of failing before schema coercion ever
// runs.
fn stringify_object_arrays(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(items) => {
            if items
                .iter()
                .any(|v| matches!(v, serde_json::Value::Object(_)))
            {
                *value = serde_json::Value::String(value.to_string());
            }
        }
        serde_json::Value::Object(fields) => {
            for v in fields.values_mut() {
                stringify_object_arrays(v);
            }
        }
        _ => {}
    }
}

impl DocBody {
    pub fn into_fields(self, id: &DocId) -> HashMap<String, Value> {
        let mut doc = self.0;
        doc.insert("_id".to_string(), id.to_string().into());
        doc
    }
}

// Shared by the bulk `update` action and the single-doc `_update` endpoint. `script` and
// `doc_as_upsert: true` are rejected by both callers — we only support merging a literal `doc`
// into an existing document, not ES's scripted-update or upsert-on-missing semantics.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateSource {
    #[serde(default)]
    pub doc: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub script: Option<serde_json::Value>,
    #[serde(default)]
    pub doc_as_upsert: Option<bool>,

    // Accepted, not honoured: controls whether/what `_source` comes back on the response; we
    // don't return a `get` field from `_update` at all yet.
    #[serde(default, rename = "_source")]
    #[allow(dead_code)]
    pub source: Option<serde_json::Value>,
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
