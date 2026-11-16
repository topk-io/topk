mod duck;
mod es;
mod file;
mod mongo;
mod topk;
mod uri;

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::Value;
use tracing::{debug, info};

use crate::import::error::Error;
use crate::import::plan::Target;

pub use duck::aws_process_profile;
use duck::Duckdb;
use es::Es;
pub use file::{File, Format, ObjectStore, Segment};
use mongo::Mongo;
use topk::Topk;
pub use uri::{SourceUrl, Uri};

#[derive(Clone)]
pub struct Table {
    pub from: SourceLocation,
    pub columns: Vec<(String, FieldSpec)>,
    pub key: Option<String>,
    /// Columns the reader had to name: a CSV column with no header cell.
    pub unnamed: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceLocation {
    Local(PathBuf),
    Remote(String),
    Object(String),
}

impl SourceLocation {
    pub fn value(&self) -> Cow<'_, str> {
        match self {
            Self::Local(path) => path.to_string_lossy(),
            Self::Remote(value) | Self::Object(value) => Cow::Borrowed(value),
        }
    }
}

impl From<&File> for SourceLocation {
    fn from(file: &File) -> Self {
        match file.store {
            None => Self::Local((&file.path).into()),
            Some(_) => Self::Remote(file.path.clone()),
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(path) => {
                let dir = std::env::current_dir().ok();
                let path = dir
                    .as_ref()
                    .and_then(|dir| path.strip_prefix(dir).ok())
                    .unwrap_or(path);
                write!(f, "{}", path.display())
            }
            Self::Remote(value) | Self::Object(value) => f.write_str(value),
        }
    }
}

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
    table: tokio::sync::OnceCell<Table>,
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

    /// Discovery lists every column the reader returns.
    pub fn typed(&self) -> bool {
        matches!(self.backend, Backend::Duck(_))
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
            table: tokio::sync::OnceCell::new(),
        }
    }

    pub async fn discover(&self) -> Result<&Table, Error> {
        self.table
            .get_or_try_init(|| async {
                info!("discovering source");
                match &self.backend {
                    Backend::Es(es) => es.discover().await,
                    Backend::Duck(duck) => duck.discover().await,
                    Backend::Mongo(mongo) => mongo.discover().await,
                    Backend::Topk(topk) => topk.discover().await,
                }
            })
            .await
    }

    /// A source may push `limit` down, but need not honor it: rows past it are
    /// cut here, and the source is not polled again once it is reached.
    pub async fn scan(
        &self,
        target: &Target,
        after: Option<&Checkpoint>,
    ) -> Result<ReadStream, Error> {
        debug!(
            columns = target.fields.len(),
            limit = target.limit,
            resumed = after.is_some(),
            "opening source scan"
        );
        let consumed = after.map_or(0, |checkpoint| checkpoint.consumed);
        let mut remaining = target.clone();
        remaining.limit = target.limit.map(|limit| limit.saturating_sub(consumed));
        if remaining.limit == Some(0) {
            return Ok(Box::pin(futures::stream::empty()));
        }
        let remaining_limit = remaining.limit;
        let cursor = after.map(|checkpoint| checkpoint.cursor.clone());
        let mut events: ReadStream = match &self.backend {
            Backend::Duck(duck @ Duckdb::Files(file)) => Box::pin(
                duck.files(file, remaining, cursor)?
                    .map_ok(ReadEvent::Chunk),
            ),
            Backend::Duck(duck) => {
                let table = self.discover().await?;
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
        let matches = match &self.backend {
            Backend::Mongo(mongo) => {
                matches!(&target.from, SourceLocation::Object(name) if name == &mongo.object)
            }
            _ => self.discover().await?.from == target.from,
        };
        if !matches {
            return Err(Error::InvalidArgument(format!(
                "source {}, saved plan reads {}; start a new import",
                self.locator, target.from
            )));
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
    use super::{Cursor, Duckdb, File, SourceLocation};
    use crate::import::plan::Target;

    #[test]
    fn cursor_shape_matches_the_source() {
        let target = Target {
            from: SourceLocation::Local("docs.parquet".into()),
            id: "_id".into(),
            fields: Default::default(),
            filter: None,
            partition: None,
            limit: None,
            exclude: Vec::new(),
        };
        let file: File = "docs.parquet".parse().unwrap();
        let error = Duckdb::Files(file.clone())
            .files(&file, target.clone(), Some(Cursor::Key("42".into())))
            .err()
            .expect("files accepted a key cursor");
        assert!(error.to_string().contains("does not fit"));
        let error = Duckdb::Sqlite {
            path: "docs.db".into(),
            object: "docs".into(),
        }
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
