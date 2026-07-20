use serde::{Deserialize, Serialize};

use super::ndjson::{NdjsonBody, NdjsonJsonHeader};
use super::search::{SearchRequest, SearchResponse};
use super::IndexName;
use crate::{Error, ErrorBody};

pub type MsearchBody = NdjsonBody<MsearchHeader>;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct MsearchHeader {
    #[serde(default)]
    index: Option<IndexName>,
}

impl NdjsonJsonHeader for MsearchHeader {
    type Payload = SearchRequest;

    fn index(&self) -> Option<IndexName> {
        self.index.clone()
    }
}

#[derive(Serialize)]
pub struct MsearchResponse {
    pub took: u32,
    pub responses: Vec<MsearchItem>,
}

impl MsearchResponse {
    pub fn new(responses: Vec<MsearchItem>) -> Self {
        Self { took: 1, responses }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum MsearchItem {
    Success {
        #[serde(flatten)]
        response: SearchResponse,
        status: u16,
    },
    Error {
        error: ErrorBody,
        status: u16,
    },
}

impl From<Result<SearchResponse, Error>> for MsearchItem {
    fn from(result: Result<SearchResponse, Error>) -> Self {
        match result {
            Ok(response) => MsearchItem::Success {
                response,
                status: 200,
            },
            Err(e) => {
                let (status, error) = e.parts();
                MsearchItem::Error { error, status }
            }
        }
    }
}
