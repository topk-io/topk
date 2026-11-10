mod catalog;
mod duck;
mod es;
mod file;
mod mongo;
mod topk;
mod uri;

use std::fmt;

use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;
use crate::import::spec::Target;

pub use catalog::{Catalog, Table};
pub use duck::aws_process_profile;
use duck::Duckdb;
use es::Es;
pub use file::{File, Format, ObjectStore};
use mongo::Mongo;
use topk::Topk;
use uri::redact;
pub use uri::Uri;

pub type Record = Vec<(String, Value)>;

/// Rows as the source reads them, each `Err` a row the source could not decode.
/// `cursor` is a source-defined position (file offset, last id, page cursor) such
/// that every row at or before it has been yielded by the end of this chunk;
/// a resumed scan continues from it.
pub struct Chunk {
    pub rows: Vec<Result<Record, Error>>,
    pub cursor: Option<Cursor>,
}

pub enum ReadEvent {
    Chunk(Chunk),
    Restarted,
}

pub(crate) type ReadStream = BoxStream<'static, Result<ReadEvent, Error>>;

type ChunkStream = BoxStream<'static, Result<Chunk, Error>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cursor {
    Key(String),
    Mongo(String),
    Sql { column: String, value: String },
    Offset { part: String, rows: u64 },
    Page { pit: String, sort: Vec<i64> },
}

impl Cursor {
    fn key(self) -> Result<String, Error> {
        match self {
            Cursor::Key(key) => Ok(key),
            other => Err(Error::InvalidArgument(format!(
                "resume cursor {other} is not a key"
            ))),
        }
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cursor::Key(value) | Cursor::Mongo(value) => f.write_str(value),
            Cursor::Sql { column, value } => write!(f, "{column}={value}"),
            Cursor::Offset { part, rows } => write!(f, "{part} (row {rows})"),
            Cursor::Page { pit, sort } => write!(f, "{pit} {sort:?}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    #[serde(flatten)]
    pub cursor: Cursor,
    #[serde(default)]
    pub consumed: u64,
}

pub struct Source {
    locator: String,
    backend: Backend,
    catalog: tokio::sync::OnceCell<Catalog>,
}

enum Backend {
    Duck(Duckdb),
    Es(Es),
    Mongo(Mongo),
    Topk(Topk),
}

impl Source {
    pub fn locator(&self) -> &str {
        &self.locator
    }

    pub fn topk(
        client: topk_rs::Client,
        locator: String,
        name: String,
        schema: std::collections::HashMap<String, FieldSpec>,
    ) -> Self {
        Self {
            locator,
            backend: Backend::Topk(Topk::new(client, name, schema)),
            catalog: tokio::sync::OnceCell::new(),
        }
    }

    pub async fn catalog(&self) -> Result<&Catalog, Error> {
        self.catalog
            .get_or_try_init(|| async {
                match &self.backend {
                    Backend::Es(es) => es.catalog().await,
                    Backend::Duck(Duckdb::Files(None)) => Ok(Catalog { tables: Vec::new() }),
                    Backend::Duck(duck) => duck.catalog().await,
                    Backend::Mongo(mongo) => Ok(Catalog {
                        tables: mongo.catalog().await?,
                    }),
                    Backend::Topk(topk) => topk.catalog().await,
                }
            })
            .await
    }

    pub(crate) fn concurrency_limit<'a>(
        &self,
        targets: impl IntoIterator<Item = &'a Target>,
    ) -> usize {
        match &self.backend {
            Backend::Duck(duck) => duck.concurrency_limit(targets),
            _ => usize::MAX,
        }
    }

    /// A source may push `limit` down, but need not honor it: rows past it are
    /// cut here, and the source is not polled again once it is reached.
    pub async fn scan(
        &self,
        target: &Target,
        after: Option<&Checkpoint>,
    ) -> Result<ReadStream, Error> {
        let consumed = after.map_or(0, |checkpoint| checkpoint.consumed);
        let mut remaining = target.clone();
        remaining.limit = target.limit.map(|limit| limit.saturating_sub(consumed));
        if remaining.limit == Some(0) {
            return Ok(Box::pin(futures::stream::empty()));
        }
        let remaining_limit = remaining.limit;
        let cursor = after.map(|checkpoint| checkpoint.cursor.clone());
        let mut events: ReadStream = match &self.backend {
            Backend::Duck(duck @ Duckdb::Files(_)) => {
                Box::pin(duck.files(remaining, cursor)?.map_ok(ReadEvent::Chunk))
            }
            Backend::Duck(duck) => {
                let table = self.catalog().await?.table(&target.from)?;
                remaining.from = table.from.clone();
                let key = table.key.clone().ok_or_else(|| {
                    Error::InvalidArgument(format!(
                        "{}: SQL imports require a single-column non-null primary or unique key",
                        target.from
                    ))
                })?;
                Box::pin(duck.table(remaining, key, cursor)?.map_ok(ReadEvent::Chunk))
            }
            Backend::Es(es) => es.chunks(target, cursor, consumed)?,
            Backend::Mongo(mongo) => {
                Box::pin(mongo.chunks(&remaining, cursor)?.map_ok(ReadEvent::Chunk))
            }
            Backend::Topk(topk) => {
                Box::pin(topk.scan(&remaining, cursor)?.map_ok(ReadEvent::Chunk))
            }
        };
        match &self.backend {
            Backend::Mongo(_) => {}
            Backend::Es(es) => {
                if self.catalog.get().is_none_or(|catalog| {
                    !catalog.tables.iter().any(|table| table.from == target.from)
                }) {
                    es.validate(&target.from).await?;
                }
            }
            _ => {
                let catalog = self.catalog().await?;
                let files;
                let catalog = if matches!(&self.backend, Backend::Duck(Duckdb::Files(_)))
                    && !catalog.tables.iter().any(|table| table.from == target.from)
                {
                    files = Duckdb::Files(Some(target.from.parse()?)).catalog().await?;
                    &files
                } else {
                    catalog
                };
                let table = if matches!(&self.backend, Backend::Duck(Duckdb::Files(_))) {
                    catalog.tables.first().ok_or_else(|| {
                        Error::InvalidArgument(format!("no source object for {:?}", target.from))
                    })?
                } else {
                    catalog.table(&target.from)?
                };
                let has = |column: &str| table.columns.iter().any(|(c, _)| c == column);
                let absent = |what: String| {
                    let available: Vec<&str> =
                        table.columns.iter().map(|(c, _)| c.as_str()).collect();
                    Error::InvalidArgument(format!(
                        "{what} is not in {:?} — available: {}",
                        target.from,
                        available.join(", ")
                    ))
                };
                if !has(&target.id) {
                    return Err(absent(format!("id column {:?}", target.id)));
                }
                if let Some(column) = target.partition.as_deref() {
                    if !has(column) {
                        return Err(absent(format!("partition column {column:?}")));
                    }
                }
                for (field, spec) in target.fields.iter() {
                    let column = spec.source(field);
                    if !has(column) {
                        return Err(absent(format!(
                            "field {field:?} reads column {column:?}, which"
                        )));
                    }
                }
            }
        }
        let limit = target.limit;
        Ok(Box::pin(async_stream::try_stream! {
            let mut left = remaining_limit;
            while left != Some(0) {
                match events.next().await {
                    Some(Ok(ReadEvent::Restarted)) => {
                        left = limit;
                        yield ReadEvent::Restarted;
                    }
                    Some(Ok(ReadEvent::Chunk(mut chunk))) => {
                        if let Some(left) = &mut left {
                            chunk.rows.truncate(*left as usize);
                            *left -= chunk.rows.len() as u64;
                        }
                        yield ReadEvent::Chunk(chunk);
                    }
                    Some(Err(error)) => Err(error)?,
                    None => break,
                }
            }
        }))
    }

    pub async fn documents(
        &self,
        target: &Target,
    ) -> Result<
        BoxStream<'static, Result<(Option<String>, topk_rs::proto::v1::data::Document), Error>>,
        Error,
    > {
        target.validate()?;
        let mut events = self.scan(target, None).await?;
        let target = target.clone();
        Ok(Box::pin(async_stream::try_stream! {
            while let Some(event) = events.try_next().await? {
                match event {
                    ReadEvent::Chunk(chunk) => {
                        for row in chunk.rows { yield target.document(row?)?; }
                    }
                    ReadEvent::Restarted => Err(Error::InvalidArgument("unexpected restart without a resume cursor".into()))?,
                }
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Duckdb};
    use crate::import::spec::Target;

    #[test]
    fn cursor_shape_matches_the_source() {
        let target = Target {
            from: "docs.parquet".to_string(),
            id: "_id".into(),
            fields: Default::default(),
            filter: None,
            partition: None,
            limit: None,
        };
        let error = Duckdb::Files(None)
            .files(target.clone(), Some(Cursor::Key("42".into())))
            .err()
            .expect("files accepted a key cursor");
        assert!(error.to_string().contains("does not fit"));
        let error = Duckdb::Sqlite("docs.db".into())
            .table(
                target,
                "_id".into(),
                Some(Cursor::Offset {
                    part: "docs.parquet".into(),
                    rows: 7,
                }),
            )
            .err()
            .expect("table accepted an offset cursor");
        assert!(error.to_string().contains("does not match source key"));
    }
}
