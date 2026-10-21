use http::StatusCode;
use serde::Serialize;

use super::{DocId, IndexName, WriteDoc};
use crate::Error;

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
    #[serde(rename = "_seq_no", skip_serializing_if = "Option::is_none")]
    pub seq_no: Option<u64>,
}

impl WriteBody {
    pub fn new(index: IndexName, id: DocId, result: WriteResult, seq_no: Option<u64>) -> Self {
        Self {
            index,
            id,
            version: 1,
            result,
            shards: super::Shards::default(),
            seq_no,
        }
    }
}

// An update that matched nothing reports an empty LSN; anything else must be a number.
pub fn parse_lsn(lsn: &str) -> Result<Option<u64>, Error> {
    if lsn.is_empty() {
        return Ok(None);
    }

    lsn.parse()
        .map(Some)
        .map_err(|_| Error::Internal(format!("invalid lsn: {lsn}")))
}
