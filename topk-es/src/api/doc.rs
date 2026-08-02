use std::collections::HashMap;
use std::fmt;

use serde::de::{Error as DeError, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use topk_rs::json::{LenientValue, Value};

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
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

pub struct DocBody(HashMap<String, Value>);

impl<'de> Deserialize<'de> for DocBody {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_map(DocBodyVisitor)?
            .map_err(DeError::custom)
    }
}

/// A document body whose field errors are captured instead of raised, so a bulk
/// request can reject the offending item and keep reading the ones around it.
/// Malformed JSON still fails the deserializer.
pub struct RawDocBody(Result<DocBody, Error>);

impl RawDocBody {
    pub fn into_body(self) -> Result<DocBody, Error> {
        self.0
    }
}

impl<'de> Deserialize<'de> for RawDocBody {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(DocBodyVisitor).map(Self)
    }
}

struct DocBodyVisitor;

impl<'de> Visitor<'de> for DocBodyVisitor {
    type Value = Result<DocBody, Error>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a document body")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut fields = HashMap::with_capacity(map.size_hint().unwrap_or(0));
        let mut rejected = None;

        // Entries keep being read after one is rejected: the body must be
        // consumed whole for the caller to report the failure as its own.
        while let Some(name) = map.next_key::<String>()? {
            let value = map.next_value::<LenientValue>()?.into_result();
            if rejected.is_some() {
                continue;
            }

            if name == "_id" {
                rejected = Some(Error::BadRequest(
                    "\"_id\" is a metadata field and cannot be set inside the document body".into(),
                ));
                continue;
            }

            match value {
                Ok(value) => {
                    fields.insert(name, Value::from(value));
                }
                Err(e) => rejected = Some(Error::BadRequest(e.to_string())),
            }
        }

        Ok(match rejected {
            Some(e) => Err(e),
            None => Ok(DocBody(fields)),
        })
    }
}

impl DocBody {
    pub fn into_fields(self, id: &DocId) -> HashMap<String, Value> {
        let mut doc = self.0;
        doc.insert("_id".to_string(), id.as_str().into());
        doc
    }
}

impl DocBody {
    #[cfg(test)]
    fn fields(&self) -> &HashMap<String, Value> {
        &self.0
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

#[cfg(test)]
mod tests {
    use topk_rs::proto::v1::data::Value as TopkValue;

    use super::*;

    #[test]
    fn reads_fields_into_topk_values() {
        let doc: DocBody =
            sonic_rs::from_str(r#"{"title": "a", "n": [1, 2], "meta": {"x": 1.5}}"#).unwrap();
        let fields = doc.fields();

        assert_eq!(fields["title"].0, TopkValue::string("a"));
        assert_eq!(fields["n"].0, TopkValue::list(vec![1_i64, 2]));
        assert_eq!(
            fields["meta"].0,
            TopkValue::r#struct([("x", TopkValue::f64(1.5))])
        );
    }

    #[test]
    fn adds_the_id_field() {
        let doc: DocBody = sonic_rs::from_str(r#"{"title": "a"}"#).unwrap();
        let id = DocId::try_from("1".to_string()).unwrap();

        assert_eq!(doc.into_fields(&id)["_id"].0, TopkValue::string("1"));
    }

    #[test]
    fn rejects_id_in_the_body() {
        assert!(sonic_rs::from_str::<DocBody>(r#"{"_id": "spoofed"}"#).is_err());
    }

    #[test]
    fn rejects_values_topk_cannot_hold() {
        assert!(sonic_rs::from_str::<DocBody>(r#"{"items": [{"a": 1}]}"#).is_err());
    }

    // Bulk reports a bad document as one failed item, so the body has to parse.
    #[rstest::rstest]
    #[case::id_in_body(r#"{"_id": "spoofed", "title": "a"}"#)]
    #[case::unrepresentable_value(r#"{"items": [{"a": 1}], "title": "a"}"#)]
    fn raw_body_captures_field_errors(#[case] body: &str) {
        let doc: RawDocBody = sonic_rs::from_str(body).expect("the line itself is valid JSON");
        assert!(doc.into_body().is_err());
    }

    #[test]
    fn raw_body_still_fails_on_malformed_json() {
        assert!(sonic_rs::from_str::<RawDocBody>(r#"{"title":"#).is_err());
    }

    #[rstest::rstest]
    #[case::array("[1, 2]")]
    #[case::scalar("5")]
    fn a_document_body_must_be_an_object(#[case] body: &str) {
        assert!(sonic_rs::from_str::<DocBody>(body).is_err());
        assert!(sonic_rs::from_str::<RawDocBody>(body).is_err());
    }
}
