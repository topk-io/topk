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
    // Optimistic-concurrency fields Kibana requires on every write response. We don't track real
    // sequence numbers, so these are constant — fine for create, but conditional updates that
    // send `if_seq_no`/`if_primary_term` won't get true conflict detection.
    #[serde(rename = "_seq_no")]
    pub seq_no: u64,
    #[serde(rename = "_primary_term")]
    pub primary_term: u64,
}

impl WriteBody {
    pub fn new(index: IndexName, id: DocId, result: WriteResult) -> Self {
        Self {
            index,
            id,
            version: 1,
            result,
            shards: super::Shards::default(),
            seq_no: 1,
            primary_term: 1,
        }
    }
}
