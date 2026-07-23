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
    LazyLock::new(|| Regex::new(r"^\.?[a-z][a-z0-9_.-]{0,126}$").unwrap());

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

    // TopK collection names admit no dots, so `.kibana` and friends are escaped. Index names are
    // lowercase (VALID_INDEX_NAME), which leaves `X` free to mark an escape.
    pub fn collection(&self) -> String {
        let mut out = String::with_capacity(self.0.len());
        for c in self.0.chars() {
            match c {
                '.' => out.push_str("XD"),
                'X' => out.push_str("XX"),
                c => out.push(c),
            }
        }
        out
    }

    pub fn from_collection(name: &str) -> Self {
        let mut out = String::with_capacity(name.len());
        let mut chars = name.chars();
        while let Some(c) = chars.next() {
            match c {
                'X' => match chars.next() {
                    Some('D') => out.push('.'),
                    Some(c) => out.push(c),
                    None => out.push('X'),
                },
                c => out.push(c),
            }
        }
        IndexName(out)
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

// ES index-scoped APIs take a comma-separated target list, not a single name.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexNames(pub Vec<IndexName>);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for IndexNames {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(IndexPath { index }) =
            Path::<IndexPath>::from_request_parts(parts, state)
                .await
                .map_err(|e| Error::BadRequest(format!("Invalid path: {e}")))?;

        index
            .split(',')
            .map(|i| IndexName::try_from(i.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map(IndexNames)
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
