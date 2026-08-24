use async_trait::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Request};
use serde::{Deserialize, Serialize};

use super::body::Body;
use super::doc::{DocItem, Source};
use super::source::SourceFilter;
use super::{DocId, IndexName};
use crate::Error;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MgetRequest {
    #[serde(default)]
    pub ids: Option<Vec<DocId>>,
    #[serde(default)]
    pub docs: Option<Vec<DocRef>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocRef {
    #[serde(rename = "_index", default)]
    pub index: Option<IndexName>,
    #[serde(rename = "_id")]
    pub id: DocId,
    #[serde(rename = "_source", default)]
    pub source: Option<SourceFilter>,
}

pub struct MgetTarget {
    pub index: IndexName,
    pub id: DocId,
    pub source: SourceFilter,
}

pub struct MgetTargets(pub Vec<MgetTarget>);

#[async_trait]
impl<S: Send + Sync> FromRequest<S> for MgetTargets {
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let (mut parts, body) = req.into_parts();
        let path_index = IndexName::from_request_parts(&mut parts, state).await.ok();
        let default_source = SourceFilter::from_request_parts(&mut parts, state).await?;

        let req = Request::from_parts(parts, body);
        let Body(request) = Body::<MgetRequest>::from_request(req, state).await?;

        let targets = match (request.ids, request.docs) {
            (Some(_), Some(_)) => {
                return Err(Error::BadRequest(
                    "Specify either \"ids\" or \"docs\", not both".into(),
                ))
            }
            (Some(ids), None) => {
                let index = path_index.ok_or_else(|| {
                    Error::BadRequest(
                        "The \"ids\" form of _mget requires an index in the path".into(),
                    )
                })?;

                ids.into_iter()
                    .map(|id| MgetTarget {
                        index: index.clone(),
                        id,
                        source: default_source.clone(),
                    })
                    .collect()
            }
            (None, Some(docs)) => docs
                .into_iter()
                .map(|doc| {
                    let index = match doc.index {
                        Some(index) => index,
                        None => path_index.clone().ok_or_else(|| {
                            Error::BadRequest("Each doc requires an \"_index\"".into())
                        })?,
                    };

                    Ok(MgetTarget {
                        index,
                        id: doc.id,
                        source: doc.source.unwrap_or_else(|| default_source.clone()),
                    })
                })
                .collect::<Result<_, Error>>()?,
            (None, None) => {
                return Err(Error::BadRequest(
                    "_mget requires \"ids\" or \"docs\"".into(),
                ))
            }
        };

        Ok(MgetTargets(targets))
    }
}

#[derive(Serialize)]
pub struct MgetBody {
    pub docs: Vec<DocItem>,
}

impl MgetBody {
    pub fn new(docs: Vec<(MgetTarget, Result<Option<Source>, Error>)>) -> Result<Self, Error> {
        let docs = docs
            .into_iter()
            .map(|(target, doc)| {
                let source = doc?;

                Ok(DocItem {
                    found: source.is_some(),
                    version: source.as_ref().map(|_| 1),
                    seq_no: source.as_ref().map(|_| 1),
                    primary_term: source.as_ref().map(|_| 1),
                    source: source.filter(|_| target.source.enabled()),
                    index: target.index,
                    id: target.id,
                })
            })
            .collect::<Result<_, Error>>()?;

        Ok(Self { docs })
    }
}
