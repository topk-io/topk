use std::fmt;
use std::ops::Deref;
use std::sync::LazyLock;

use async_trait::async_trait;
use axum::extract::{FromRequestParts, Path};
use http::request::Parts;
use regex::Regex;
use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::Error;

static VALID_INDEX_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_-]{0,127}$").unwrap());

#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct IndexName(String);

impl TryFrom<String> for IndexName {
    type Error = Error;

    fn try_from(index: String) -> Result<Self, Self::Error> {
        if !VALID_INDEX_NAME.is_match(&index) {
            return Err(Error::InvalidIndexName(format!(
                "\"{index}\": must start with a lowercase letter and contain only lowercase \
                 letters, digits, underscores, and dashes (max 128 characters)"
            )));
        }

        Ok(Self(index))
    }
}

impl IndexName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IndexName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for IndexName {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

#[derive(Deserialize)]
struct IndexPath {
    index: String,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for IndexName {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(IndexPath { index }) =
            Path::<IndexPath>::from_request_parts(parts, state)
                .await
                .map_err(|e| Error::BadRequest(format!("Invalid path: {e}")))?;

        IndexName::try_from(index)
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DocId(String);

// ES coerces numeric ids to their string form, so accept a string or a number.
impl<'de> Deserialize<'de> for DocId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(DocIdVisitor)
    }
}

struct DocIdVisitor;

impl DocIdVisitor {
    fn build<E: DeError>(id: String) -> Result<DocId, E> {
        DocId::try_from(id).map_err(DeError::custom)
    }
}

impl Visitor<'_> for DocIdVisitor {
    type Value = DocId;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a document id as a string or number")
    }

    fn visit_str<E: DeError>(self, id: &str) -> Result<DocId, E> {
        Self::build(id.to_string())
    }

    fn visit_string<E: DeError>(self, id: String) -> Result<DocId, E> {
        Self::build(id)
    }

    fn visit_i64<E: DeError>(self, id: i64) -> Result<DocId, E> {
        Self::build(id.to_string())
    }

    fn visit_u64<E: DeError>(self, id: u64) -> Result<DocId, E> {
        Self::build(id.to_string())
    }

    fn visit_f64<E: DeError>(self, id: f64) -> Result<DocId, E> {
        Self::build(id.to_string())
    }
}

impl TryFrom<String> for DocId {
    type Error = Error;

    fn try_from(id: String) -> Result<Self, Self::Error> {
        if id.is_empty() {
            return Err(Error::InvalidDocId("Document id must not be empty".into()));
        }
        if id.len() > 512 {
            return Err(Error::InvalidDocId(format!(
                "Document id is too long, must be no longer than 512 bytes, got {}",
                id.len()
            )));
        }
        Ok(Self(id))
    }
}

impl DocId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for DocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for DocId {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::string(r#""abc""#, "abc")]
    #[case::integer("42", "42")]
    #[case::negative_integer("-42", "-42")]
    #[case::float("1.5", "1.5")]
    fn doc_id_accepts_strings_and_numbers(#[case] body: &str, #[case] expected: &str) {
        let id: DocId = sonic_rs::from_str(body).unwrap();
        assert_eq!(id.as_str(), expected);
    }

    #[rstest]
    #[case::empty(r#""""#)]
    #[case::bool("true")]
    #[case::null("null")]
    #[case::array("[1]")]
    fn doc_id_rejected(#[case] body: &str) {
        assert!(sonic_rs::from_str::<DocId>(body).is_err());
    }

    #[test]
    fn doc_id_rejects_ids_over_512_bytes() {
        let id = "a".repeat(513);
        assert!(sonic_rs::from_str::<DocId>(&format!("\"{id}\"")).is_err());
    }
}

#[derive(Deserialize)]
struct DocPath {
    id: String,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for DocId {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(DocPath { id }) = Path::<DocPath>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::BadRequest(format!("Invalid path: {e}")))?;

        DocId::try_from(id)
    }
}
