mod codec;
mod duck;
mod es;
mod mongo;
mod topk;
mod uri;

use std::fmt;

use futures::stream::BoxStream;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;
use crate::import::spec::{Field, Target};
use crate::import::ID_PLACEHOLDER;

pub use duck::{aws_process_profile, Duckdb, File};
use es::Es;
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

pub type ChunkStream = BoxStream<'static, Result<Chunk, Error>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cursor {
    Key(String),
    Offset { part: String, rows: u64 },
    Page { pit: String, sort: Vec<i64> },
}

impl Cursor {
    pub fn key(self) -> Result<String, Error> {
        match self {
            Cursor::Key(key) => Ok(key),
            other => Err(Error::InvalidArgument(format!(
                "resume cursor {other} is not a key"
            ))),
        }
    }

    pub fn offset(self) -> Result<(String, u64), Error> {
        match self {
            Cursor::Offset { part, rows } => Ok((part, rows)),
            other => Err(Error::InvalidArgument(format!(
                "resume cursor {other} is not an offset"
            ))),
        }
    }

    pub fn page(self) -> Result<(String, Vec<i64>), Error> {
        match self {
            Cursor::Page { pit, sort } => Ok((pit, sort)),
            other => Err(Error::InvalidArgument(format!(
                "resume cursor {other} is not a page"
            ))),
        }
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cursor::Key(value) => f.write_str(value),
            Cursor::Offset { part, rows } => write!(f, "{part} (row {rows})"),
            Cursor::Page { pit, sort } => write!(f, "{pit} {sort:?}"),
        }
    }
}

#[derive(Clone)]
pub struct Table {
    pub from: String,
    pub collection_hint: Option<String>,
    pub columns: Vec<(String, Field)>,
    pub primary_key: Option<String>,
}

pub struct Source {
    pub locator: String,
    backend: Backend,
}

enum Backend {
    Duck(Duckdb),
    Es(Es),
    Mongo(Mongo),
    Topk(Topk),
}

impl Source {
    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        match &self.backend {
            Backend::Duck(duck) => duck.catalog().await,
            Backend::Es(es) => es.catalog().await,
            Backend::Mongo(mongo) => mongo.catalog().await,
            Backend::Topk(topk) => Ok(vec![topk.table.clone()]),
        }
    }

    pub async fn validate<'a>(
        &self,
        targets: impl Iterator<Item = (&'a String, &'a Target)>,
        catalog: Option<&[Table]>,
    ) -> Result<(), Error> {
        if matches!(&self.backend, Backend::Mongo(_)) {
            return Ok(());
        }
        let fetched;
        let catalog = match catalog {
            Some(catalog) => catalog,
            None => {
                fetched = self.catalog().await?;
                &fetched
            }
        };

        for (name, target) in targets {
            let table = match catalog.iter().find(|table| table.from == target.from) {
                Some(table) => table,
                None => continue,
            };
            let has = |column: &str| table.columns.iter().any(|(c, _)| c == column);
            let absent = |what: String| {
                let available: Vec<&str> = table.columns.iter().map(|(c, _)| c.as_str()).collect();
                Error::InvalidArgument(format!(
                    "{name}: {what} is not in {:?} — available: {}",
                    target.from,
                    available.join(", ")
                ))
            };
            // A placeholder id is "not detected", tolerated so --dry-run can show it.
            if let Some(id) = target.id.as_deref() {
                if id != ID_PLACEHOLDER && !has(id) {
                    return Err(absent(format!("id column {id:?}")));
                }
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
        Ok(())
    }

    pub fn concurrency_limit<'a>(&self, targets: impl IntoIterator<Item = &'a Target>) -> usize {
        match &self.backend {
            Backend::Duck(duck) => duck.concurrency_limit(targets),
            _ => usize::MAX,
        }
    }

    /// A source may push `limit` down, but need not honor it: rows past it are
    /// cut here, and the source is not polled again once it is reached.
    pub fn scan(&self, target: &Target, after: Option<Cursor>) -> Result<ChunkStream, Error> {
        let mut chunks = match &self.backend {
            Backend::Duck(duck) => duck.chunks(
                target,
                target.filter.as_deref().map(str::parse).transpose()?,
                after.map(TryInto::try_into).transpose()?,
            )?,
            Backend::Es(es) => es.chunks(
                target,
                target.filter.as_deref().map(str::parse).transpose()?,
                after.map(TryInto::try_into).transpose()?,
            ),
            Backend::Mongo(mongo) => mongo.chunks(
                target,
                target.filter.as_deref().map(str::parse).transpose()?,
                after.map(TryInto::try_into).transpose()?,
            ),
            Backend::Topk(topk) => topk.scan(target, after)?,
        };
        let mut left = match target.limit {
            Some(limit) => limit,
            None => return Ok(chunks),
        };
        Ok(Box::pin(async_stream::try_stream! {
            while left > 0 {
                let mut chunk = match chunks.next().await {
                    Some(chunk) => chunk?,
                    None => break,
                };
                chunk.rows.truncate(left as usize);
                left -= chunk.rows.len() as u64;
                yield chunk;
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Backend, Cursor, Duckdb, Source};
    use crate::import::Target;

    #[test]
    fn cursor_shape_matches_the_source() {
        let target = Target {
            from: "docs.parquet".to_string(),
            ..Default::default()
        };
        let error = Source {
            locator: String::new(),
            backend: Backend::Duck(Duckdb::Files(None)),
        }
        .scan(&target, Some(Cursor::Key("42".to_string())))
        .err()
        .expect("files accepted a key cursor");
        assert_eq!(
            error.to_string(),
            "resume cursor Key(\"42\") does not fit docs.parquet"
        );

        let offset = Cursor::Offset {
            part: "docs.parquet".to_string(),
            rows: 7,
        };
        let error = Source {
            locator: String::new(),
            backend: Backend::Duck(Duckdb::Sqlite("docs.db".to_string())),
        }
        .scan(&target, Some(offset))
        .err()
        .expect("table accepted an offset cursor");
        assert_eq!(
            error.to_string(),
            "resume cursor Offset { part: \"docs.parquet\", rows: 7 } does not fit docs.parquet"
        );
    }
}
