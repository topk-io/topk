use async_trait::async_trait;
use axum::extract::{FromRequestParts, Query};
use http::request::Parts;
use serde::Deserialize;

use crate::Error;

// TopK extension, not Elasticsearch: pins a read to an earlier write's `_seq_no`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
pub struct RequiredLsn(pub Option<u64>);

#[derive(Deserialize)]
struct RequiredLsnQuery {
    #[serde(default)]
    required_lsn: Option<u64>,
}

impl From<RequiredLsn> for Option<String> {
    fn from(lsn: RequiredLsn) -> Self {
        lsn.0.map(|lsn| lsn.to_string())
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for RequiredLsn {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<RequiredLsnQuery>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::BadRequest(format!("Invalid query string: {e}")))?;

        Ok(RequiredLsn(query.required_lsn))
    }
}
