use std::fmt;
use std::ops::Deref;
use std::sync::LazyLock;

use async_trait::async_trait;
use axum::extract::{FromRequestParts, Path};
use http::request::Parts;
use regex::Regex;
use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};

use crate::Error;

static VALID_INDEX_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$").unwrap());

#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct IndexName(String);

impl TryFrom<String> for IndexName {
    type Error = Error;

    fn try_from(index: String) -> Result<Self, Self::Error> {
        if !VALID_INDEX_NAME.is_match(&index) {
            return Err(Error::InvalidIndexName(format!(
                "\"{index}\": must start with a letter or digit and contain only letters, \
                 digits, underscores, dashes, and dots (max 255 characters)"
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
        let id = match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            other => {
                return Err(DeError::custom(format!(
                    "document id must be a string or number, got {other}"
                )))
            }
        };
        DocId::try_from(id).map_err(DeError::custom)
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

#[derive(Deserialize)]
struct DocPath {
    id: String,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::plain("books")]
    #[case::dashed("books-v2")]
    #[case::underscored("books_v2")]
    #[case::dated_logs("logs-2026.07.29")]
    #[case::leading_digit("2026-logs")]
    #[case::all_digits("1234")]
    #[case::uppercase("Books")]
    #[case::trailing_dot("logs-2026.07.29.")]
    #[case::max_length(&"a".repeat(255))]
    fn index_name_ok(#[case] index: &str) {
        assert_eq!(
            IndexName::try_from(index.to_string()).unwrap().as_str(),
            index
        );
    }

    #[rstest]
    #[case::empty("")]
    #[case::leading_dash("-books")]
    #[case::leading_underscore("_books")]
    #[case::leading_dot(".books")]
    #[case::plus("books+v2")]
    #[case::space("books v2")]
    #[case::comma("books,logs")]
    #[case::wildcard("books*")]
    #[case::too_long(&"a".repeat(256))]
    fn index_name_rejected(#[case] index: &str) {
        assert!(IndexName::try_from(index.to_string()).is_err());
    }
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
