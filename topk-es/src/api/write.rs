use http::StatusCode;
use serde::Serialize;

use super::{DocId, IndexName, WriteDoc};

pub enum WriteRequest {
    Upsert(Vec<WriteDoc>),
    Update(Vec<WriteDoc>),
    Delete(Vec<DocId>),
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WriteResult {
    Created,
    Updated,
    Deleted,
}

impl WriteResult {
    pub fn status_code(self) -> StatusCode {
        match self {
            WriteResult::Created => StatusCode::CREATED,
            WriteResult::Updated | WriteResult::Deleted => StatusCode::OK,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct WriteBody {
    #[serde(rename = "_index")]
    pub index: IndexName,
    #[serde(rename = "_id")]
    pub id: DocId,
    #[serde(rename = "_version")]
    pub version: u32,
    pub result: WriteResult,
    #[serde(rename = "_shards")]
    pub shards: super::Shards,
}

impl WriteBody {
    pub fn new(index: IndexName, id: DocId, result: WriteResult) -> Self {
        Self {
            index,
            id,
            version: 1,
            result,
            shards: super::Shards::default(),
        }
    }
}
