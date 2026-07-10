use std::fmt;
use std::ops::Deref;
use std::sync::LazyLock;

use async_trait::async_trait;
use axum::extract::{FromRequestParts, Path};
use http::request::Parts;
use regex::Regex;
use serde::{Deserialize, Serialize};

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
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(try_from = "String")]
pub struct DocId(String);

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
