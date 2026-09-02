mod codec;
mod duck;
mod es;
mod mongo;
mod topk;
mod uri;

use std::fmt;

use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use topk_rs::proto::v1::data::Value;

use crate::endpoint::Endpoint;
use crate::import::error::Error;
use crate::import::spec::{Field, Target};

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

pub struct Scan {
    pub target: Target,
    pub chunks: ChunkStream,
}

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

pub enum Source {
    Duck(Duckdb),
    Es(Es),
    Mongo(Mongo),
    Topk(Topk),
}

impl Source {
    pub async fn connect(uri: &Uri, endpoint: &Endpoint) -> Result<Source, Error> {
        Ok(match uri {
            Uri::Duck(duck) => Source::Duck(duck.clone()),
            Uri::Es(url) => Source::Es(Es::new(url.clone())?),
            Uri::Mongo(url) => Source::Mongo(Mongo::connect(url).await?),
            Uri::Topk(uri) => Source::Topk(Topk::connect(uri, endpoint)?),
        })
    }

    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        match self {
            Source::Duck(duck) => duck.catalog().await,
            Source::Es(es) => es.catalog().await,
            Source::Mongo(mongo) => mongo.catalog().await,
            Source::Topk(topk) => topk.catalog().await,
        }
    }

    /// Whether the catalog lists every column a scan can yield. True for schema
    /// sources (files/SQL via `SELECT *`, ES mappings, topk); false for mongodb,
    /// whose catalog is a document sample — a rare field may not appear in it, so
    /// a spec column absent from the catalog is not proof the source lacks it.
    pub fn columns_are_exhaustive(&self) -> bool {
        !matches!(self, Source::Mongo(_))
    }

    pub fn concurrency_limit(&self) -> usize {
        match self {
            Source::Duck(duck) => duck.concurrency_limit(),
            _ => usize::MAX,
        }
    }

    pub fn scan(&self, target: &Target, after: Option<Cursor>) -> Result<Scan, Error> {
        let chunks = match self {
            Source::Duck(duck) => duck.chunks(
                target,
                target.parsed_filter()?,
                after.map(TryInto::try_into).transpose()?,
            )?,
            Source::Es(es) => es.chunks(
                target,
                target.parsed_filter()?,
                after.map(TryInto::try_into).transpose()?,
            ),
            Source::Mongo(mongo) => mongo.chunks(
                target,
                target.parsed_filter()?,
                after.map(TryInto::try_into).transpose()?,
            ),
            Source::Topk(topk) => topk.chunks(
                target,
                target.parsed_filter()?,
                after.map(TryInto::try_into).transpose()?,
            ),
        };
        Ok(Scan {
            target: target.clone(),
            chunks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Duckdb, Source};
    use crate::import::Target;

    #[test]
    fn cursor_shape_matches_the_source() {
        let target = Target {
            from: "docs.parquet".to_string(),
            ..Default::default()
        };
        let error = Source::Duck(Duckdb::Files(None))
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
        let error = Source::Duck(Duckdb::Sqlite("docs.db".to_string()))
            .scan(&target, Some(offset))
            .err()
            .expect("table accepted an offset cursor");
        assert_eq!(
            error.to_string(),
            "resume cursor Offset { part: \"docs.parquet\", rows: 7 } does not fit docs.parquet"
        );
    }
}
