pub mod codec;
mod duck;
mod es;
mod mongo;
mod topk;
pub mod uri;

use futures::stream::BoxStream;
use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;
use crate::import::source::uri::Uri;
use crate::import::spec::{Field, Target};

pub use duck::{max_readers, Duckdb, READ_AHEAD};

use es::Es;
use mongo::Mongo;
use topk::Topk;

pub type Record = Vec<(String, Value)>;

/// Rows as the source reads them. `mark` is a source-defined position (file
/// offset, last id, page cursor) such that every row at or before it has been
/// yielded by the end of this chunk; `stream(after)` continues from it.
pub struct Chunk {
    pub rows: Vec<Result<Record, Error>>,
    pub mark: Option<String>,
}
pub type Records = BoxStream<'static, Chunk>;

pub enum Source {
    Duck(Duckdb),
    Es(Es),
    Mongo(Mongo),
    Topk(Topk),
}

impl Source {
    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        match self {
            Source::Duck(duck) => duck.catalog().await,
            Source::Es(es) => es.catalog().await,
            Source::Mongo(mongo) => mongo.catalog().await,
            Source::Topk(topk) => topk.catalog().await,
        }
    }

    /// Rows of `target` after `after`, ordered so a chunk's mark is a resume
    /// point. A stale cursor the source cannot honour is `Error::Expired`.
    pub async fn stream(&self, target: &Target, after: Option<&str>) -> Result<Records, Error> {
        match self {
            Source::Duck(duck) => duck.stream(target, after).await,
            Source::Es(es) => es.stream(target, after).await,
            Source::Mongo(mongo) => mongo.stream(target, after).await,
            Source::Topk(topk) => topk.stream(target, after).await,
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

pub async fn connect(uri: &Uri) -> Result<Source, Error> {
    Ok(match uri {
        Uri::Postgres(url) => Source::Duck(Duckdb::Postgres(url.as_str().to_string())),
        Uri::Mysql(url) => Source::Duck(Duckdb::Mysql(url.as_str().to_string())),
        Uri::Sqlite(path) => Source::Duck(Duckdb::Sqlite(path.clone())),
        Uri::Elasticsearch(url) => Source::Es(Es::new(url.clone())?),
        Uri::Mongo(url) => Source::Mongo(Mongo::connect(url.as_str()).await?),
        Uri::Topk {
            region,
            host,
            https,
            collection,
        } => Source::Topk(Topk::new(region, host.as_deref(), *https, collection)?),
        Uri::File { path, .. } => Source::Duck(Duckdb::Files(Some(path.clone()))),
    })
}
