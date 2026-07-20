use async_trait::async_trait;
use axum::extract::{FromRequestParts, Query};
use http::request::Parts;
use serde::Deserialize;

use crate::Error;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Refresh {
    #[default]
    False,

    #[serde(rename = "wait_for")]
    WaitFor,

    #[serde(alias = "")]
    True,
}

impl Refresh {
    pub fn is_blocking(&self) -> bool {
        matches!(self, Refresh::WaitFor | Refresh::True)
    }
}

#[derive(Deserialize)]
struct RefreshQuery {
    #[serde(default)]
    refresh: Refresh,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Refresh {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(query) = Query::<RefreshQuery>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::BadRequest(format!("Invalid query string: {e}")))?;

        Ok(query.refresh)
    }
}
