use futures::TryStreamExt;

use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, filter, SortOrder};

use crate::import::error::Error;
use crate::import::plan::Target;
use crate::import::source::Record;
use crate::import::ID;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::Client;

use super::{Chunk, ChunkStream, Cursor as Checkpoint, SourceLocation, Table};

#[derive(Clone)]
pub struct Topk {
    client: Client,
    pub table: Table,
}

impl Topk {
    pub fn new(
        client: Client,
        name: String,
        schema: std::collections::HashMap<String, topk_rs::proto::v1::control::FieldSpec>,
    ) -> Topk {
        let table = Table {
            columns: std::iter::once((ID.to_string(), FieldSpec::text(false)))
                .chain(schema)
                .collect(),
            from: SourceLocation::Object(name),
            key: Some(ID.to_string()),
        };
        Topk { client, table }
    }

    pub async fn discover(&self) -> Result<Table, Error> {
        if self
            .client
            .collection(self.table.from.value())
            .list_partitions(None)
            .await?
            .try_next()
            .await?
            .is_some()
        {
            return Err(Error::InvalidArgument(format!(
                "collection {:?} has named partitions; copying partitioned collections is not supported",
                self.table.from
            )));
        }
        Ok(self.table.clone())
    }

    /// Keyset pages ordered by `_id`, so a page boundary is a resume point.
    /// `fetch` and not `select`: a select of an indexed vector is refused by the
    /// server, and returns the index's quantized copy where it is not.
    pub fn scan(&self, target: &Target, after: Option<Checkpoint>) -> Result<ChunkStream, Error> {
        if target.filter.is_some() {
            return Err(Error::InvalidArgument(
                "--filter is not supported for topk:// sources".into(),
            ));
        }
        let mut cursor = after.map(Checkpoint::key).transpose()?.unwrap_or_default();
        let collection = self.client.collection(target.from.value());
        // 10k is the server's cap on a sorted `limit`, and the resume granularity.
        let size = target.limit.map_or(10_000, |limit| limit.min(10_000));

        let stream = async_stream::try_stream! {
            loop {
                let documents = collection
                    .query(
                        filter(field(ID).gt(cursor.clone()))
                            .sort((field(ID), SortOrder::Asc))
                            .limit(size)
                            .fetch(["*"]),
                        None,
                        None,
                    )
                    .await?;
                if documents.is_empty() {
                    break;
                }
                let last = documents
                    .last()
                    .and_then(|document| document.fields.get(ID))
                    .and_then(Value::as_string)
                    .ok_or_else(|| Error::InvalidArgument("TopK page has no valid continuation id".into()))?;
                cursor = last.to_string();
                let full = documents.len() as u64 == size;
                yield Chunk {
                    rows: documents
                        .into_iter()
                        .map(|document| Ok(document.fields.into_iter().collect::<Record>()))
                        .collect(),
                    cursor: Some(Checkpoint::Key(cursor.clone())),
                };
                if !full {
                    break;
                }
            }
        };
        Ok(Box::pin(stream))
    }
}
