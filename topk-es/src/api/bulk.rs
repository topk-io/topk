use std::collections::HashMap;

use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use super::ndjson::{NdjsonBody, NdjsonHeader, NdjsonLines};
use super::{
    DocBody, DocId, IndexName, UpdateSource, WriteBody, WriteDoc, WriteRequest, WriteResult,
};
use crate::{Error, ErrorBody};

#[derive(Clone, Copy)]
pub enum WriteKind {
    Index,
    Create,
    Update,
    Delete,
}

impl WriteKind {
    fn as_str(self) -> &'static str {
        match self {
            WriteKind::Index => "index",
            WriteKind::Create => "create",
            WriteKind::Update => "update",
            WriteKind::Delete => "delete",
        }
    }
}

pub struct BulkEntry {
    pub id: DocId,
    pub kind: WriteKind,
    pub request: Result<WriteRequest, Error>,
}

impl BulkEntry {
    fn unsupported(id: DocId, kind: WriteKind, msg: impl Into<String>) -> Self {
        Self {
            id,
            kind,
            request: Err(Error::Unsupported(msg.into())),
        }
    }
}

pub struct BulkRef {
    pub index: IndexName,
    pub id: DocId,
    pub kind: WriteKind,
}

pub type BulkBody = NdjsonBody<ActionLine>;

impl NdjsonHeader for ActionLine {
    type Payload = BulkEntry;

    fn index(&self) -> Option<IndexName> {
        self.meta().index.clone()
    }

    fn parse_payload(self, lines: &mut NdjsonLines<'_>) -> Result<BulkEntry, Error> {
        let id = self.meta().id.clone().ok_or_else(|| {
            Error::Unsupported("\"_id\" is required; auto-generated ids are not supported".into())
        })?;

        match self {
            ActionLine::Index(_) => {
                let doc: HashMap<String, serde_json::Value> = lines.parse()?;
                let request = DocBody::try_from(doc)
                    .map(|body| WriteRequest::Upsert(vec![WriteDoc::new(id.clone(), body)]));
                Ok(BulkEntry {
                    id,
                    kind: WriteKind::Index,
                    request,
                })
            }
            ActionLine::Create(_) => {
                let _: serde_json::Value = lines.parse()?;
                Ok(BulkEntry::unsupported(
                    id,
                    WriteKind::Create,
                    "\"create\" is not supported — TopK has no fail-if-exists semantics",
                ))
            }
            ActionLine::Update(_) => {
                let src: UpdateSource = lines.parse()?;
                if src.script.is_some() {
                    return Ok(BulkEntry::unsupported(
                        id,
                        WriteKind::Update,
                        "Scripted updates are not supported",
                    ));
                }
                if src.doc_as_upsert == Some(true) {
                    return Ok(BulkEntry::unsupported(
                        id,
                        WriteKind::Update,
                        "[doc_as_upsert] is not supported",
                    ));
                }
                let doc = src
                    .doc
                    .ok_or_else(|| Error::BadRequest("Update action missing \"doc\"".into()))?;
                let request = DocBody::try_from(doc)
                    .map(|body| WriteRequest::Update(vec![WriteDoc::new(id.clone(), body)]));
                Ok(BulkEntry {
                    id,
                    kind: WriteKind::Update,
                    request,
                })
            }
            ActionLine::Delete(_) => Ok(BulkEntry {
                id: id.clone(),
                kind: WriteKind::Delete,
                request: Ok(WriteRequest::Delete(vec![id])),
            }),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionLine {
    Index(ActionMeta),
    Create(ActionMeta),
    Update(ActionMeta),
    Delete(ActionMeta),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionMeta {
    #[serde(rename = "_index", default)]
    index: Option<IndexName>,
    #[serde(rename = "_id", default)]
    id: Option<DocId>,
}

impl ActionLine {
    fn meta(&self) -> &ActionMeta {
        match self {
            ActionLine::Index(meta)
            | ActionLine::Create(meta)
            | ActionLine::Update(meta)
            | ActionLine::Delete(meta) => meta,
        }
    }
}

#[derive(Clone)]
pub struct BulkItem {
    pub kind: WriteKind,
    pub result: BulkItemResult,
}

impl Serialize for BulkItem {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(self.kind.as_str(), &self.result)?;
        map.end()
    }
}

#[derive(Clone, Serialize)]
#[serde(untagged)]
pub enum BulkItemResult {
    Success {
        #[serde(flatten)]
        body: WriteBody,
        status: u16,
    },
    Error {
        #[serde(rename = "_index")]
        index: IndexName,
        #[serde(rename = "_id")]
        id: DocId,
        status: u16,
        error: ErrorBody,
    },
}

impl From<(IndexName, DocId, Result<WriteResult, Error>)> for BulkItemResult {
    fn from((index, id, result): (IndexName, DocId, Result<WriteResult, Error>)) -> Self {
        match result {
            Ok(result) => BulkItemResult::Success {
                status: result.status_code().as_u16(),
                body: WriteBody::new(index, id, result),
            },
            Err(error) => {
                let (status, error) = error.parts();
                BulkItemResult::Error {
                    index,
                    id,
                    status,
                    error,
                }
            }
        }
    }
}

#[derive(Serialize)]
pub struct BulkResponse {
    pub took: u32,
    pub errors: bool,
    pub items: Vec<BulkItem>,
}

impl BulkResponse {
    pub fn new(results: Vec<(BulkRef, Result<String, Error>)>) -> Self {
        let items: Vec<BulkItem> = results
            .into_iter()
            .map(|(line, result)| {
                let result = result.map(|_| match line.kind {
                    WriteKind::Index | WriteKind::Create => WriteResult::Created,
                    WriteKind::Update => WriteResult::Updated,
                    WriteKind::Delete => WriteResult::Deleted,
                });
                BulkItem {
                    kind: line.kind,
                    result: (line.index, line.id, result).into(),
                }
            })
            .collect();

        let errors = items
            .iter()
            .any(|item| matches!(item.result, BulkItemResult::Error { .. }));

        Self {
            took: 1,
            errors,
            items,
        }
    }
}
