use async_trait::async_trait;
use axum::extract::{FromRequestParts, Query};
use http::request::Parts;
use serde::Deserialize;

use crate::Error;

// `ignore_unavailable=true` turns a missing index into an empty result instead of
// a `404`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(transparent)]
pub struct IgnoreUnavailable(bool);

impl IgnoreUnavailable {
    pub fn is_set(&self) -> bool {
        self.0
    }
}

#[derive(Deserialize)]
struct IgnoreUnavailableQuery {
    #[serde(default)]
    ignore_unavailable: IgnoreUnavailable,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for IgnoreUnavailable {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<IgnoreUnavailableQuery>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::BadRequest(format!("Invalid query string: {e}")))?;

        Ok(query.ignore_unavailable)
    }
}
