use async_trait::async_trait;
use axum::extract::{FromRequestParts, Query};
use http::request::Parts;
use serde::Deserialize;

use crate::Error;

#[derive(Debug, Clone, Deserialize)]
#[serde(from = "SourceFilterWire")]
pub struct SourceFilter {
    enabled: bool,
    includes: Vec<String>,
    excludes: Vec<String>,
}

impl Default for SourceFilter {
    fn default() -> Self {
        Self::new(true, Vec::new(), Vec::new())
    }
}

impl SourceFilter {
    pub fn new(enabled: bool, includes: Vec<String>, excludes: Vec<String>) -> Self {
        Self {
            enabled,
            includes,
            excludes,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn keep(&self, path: &str) -> bool {
        if self.excludes.iter().any(|p| Self::matches(p, path)) {
            return false;
        }
        self.includes.is_empty()
            || self.includes.iter().any(|p| p == "*")
            || self.includes.iter().any(|p| Self::matches(p, path))
    }

    fn matches(pattern: &str, path: &str) -> bool {
        let pattern = pattern.strip_suffix(".*").unwrap_or(pattern);
        path == pattern || path.starts_with(&format!("{pattern}."))
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SourceFilterWire {
    Enabled(bool),
    Includes(Vec<String>),
    Include(String),
    Filter(SourceFilterWireFields),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFilterWireFields {
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    excludes: Vec<String>,
}

impl From<SourceFilterWire> for SourceFilter {
    fn from(wire: SourceFilterWire) -> Self {
        match wire {
            SourceFilterWire::Enabled(enabled) => {
                SourceFilter::new(enabled, Vec::new(), Vec::new())
            }
            SourceFilterWire::Includes(includes) => SourceFilter::new(true, includes, Vec::new()),
            SourceFilterWire::Include(field) => SourceFilter::new(true, vec![field], Vec::new()),
            SourceFilterWire::Filter(SourceFilterWireFields { includes, excludes }) => {
                SourceFilter::new(true, includes, excludes)
            }
        }
    }
}

#[derive(Deserialize)]
struct SourceQueryParams {
    #[serde(rename = "_source")]
    source: Option<String>,
    #[serde(rename = "_source_includes")]
    source_includes: Option<String>,
    #[serde(rename = "_source_excludes")]
    source_excludes: Option<String>,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for SourceQueryParams {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(params) = Query::<Self>::from_request_parts(parts, state)
            .await
            .map_err(|e| Error::BadRequest(format!("Invalid query string: {e}")))?;
        Ok(params)
    }
}

impl SourceQueryParams {
    fn is_empty(&self) -> bool {
        self.source.is_none() && self.source_includes.is_none() && self.source_excludes.is_none()
    }

    fn into_filter(self) -> SourceFilter {
        fn split_csv(s: &str) -> Vec<String> {
            s.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        }

        let mut includes = self
            .source_includes
            .as_deref()
            .map(split_csv)
            .unwrap_or_default();
        let excludes = self
            .source_excludes
            .as_deref()
            .map(split_csv)
            .unwrap_or_default();

        let enabled = match self.source.as_deref() {
            None | Some("true") => true,
            Some("false") => false,
            Some(csv) => {
                includes.extend(split_csv(csv));
                true
            }
        };

        SourceFilter::new(enabled, includes, excludes)
    }
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for SourceFilter {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(SourceQueryParams::from_request_parts(parts, state)
            .await?
            .into_filter())
    }
}

/// The `_source*` query params, or `None` if the request carried none.
///
/// `SourceFilter`'s own extractor cannot say "absent" -- with no params it yields the
/// default, which reads as an explicit "everything".
#[derive(Debug, Clone)]
pub struct SourceQuery(pub Option<SourceFilter>);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for SourceQuery {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let params = SourceQueryParams::from_request_parts(parts, state).await?;
        Ok(SourceQuery((!params.is_empty()).then(|| params.into_filter())))
    }
}

/// Rejects `_source*` query params, which Elasticsearch does not accept on every
/// endpoint that takes a search body -- `_msearch` answers 400 for all three.
pub struct NoSourceQuery;

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for NoSourceQuery {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let path = parts.uri.path().to_string();
        match SourceQueryParams::from_request_parts(parts, state).await?.is_empty() {
            true => Ok(Self),
            false => Err(Error::Unsupported(format!(
                "request [{path}] contains unrecognized parameter: [_source*]"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SourceFilter;
    use serde_json::json;

    #[test]
    fn string_include_filters_to_one_field() {
        let filter: SourceFilter = serde_json::from_value(json!("title")).unwrap();
        assert!(filter.keep("title"));
        assert!(!filter.keep("genre"));
    }

    #[test]
    fn star_include_matches_all_fields() {
        let filter = SourceFilter::new(true, vec!["*".into()], vec![]);
        assert!(filter.keep("title"));
        assert!(filter.keep("meta.author"));
    }

    #[test]
    fn star_include_still_respects_excludes() {
        let filter = SourceFilter::new(true, vec!["*".into()], vec!["title".into()]);
        assert!(!filter.keep("title"));
        assert!(filter.keep("genre"));
    }
}
