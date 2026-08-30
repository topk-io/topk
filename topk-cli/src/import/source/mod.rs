mod codec;
mod duck;
mod es;
mod mongo;
mod topk;
mod uri;

use futures::stream::BoxStream;
use mongodb::bson::{doc, to_document, Document as BsonDoc};
use serde_json::{json, Value as JsonValue};
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
/// `mark` is a source-defined position (file offset, last id, page cursor) such
/// that every row at or before it has been yielded by the end of this chunk;
/// `stream(after)` continues from it.
pub struct Chunk {
    pub rows: Vec<Result<Record, Error>>,
    pub mark: Option<String>,
}

pub type Records = BoxStream<'static, Result<Chunk, Error>>;

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

    pub fn scan(&self, name: &str, target: &Target, after: Option<&str>) -> Result<Scan, Error> {
        let filter = target.filter.as_deref();
        let read = match self {
            Source::Duck(duck) => Read::Duck(
                duck.clone(),
                match duck {
                    Duckdb::Files(_) => Some(target.from.parse()?),
                    _ => None,
                },
            ),
            Source::Es(es) => Read::Es(
                es.clone(),
                match filter {
                    Some(f) => serde_json::from_str(f).map_err(|e| {
                        Error::InvalidArgument(format!(
                            "filter {f:?} is not JSON — elasticsearch filters are query DSL \
                             objects, e.g. '{{\"range\": {{\"year\": {{\"gt\": 2000}}}}}}' ({e})"
                        ))
                    })?,
                    None => json!({ "match_all": {} }),
                },
            ),
            Source::Mongo(mongo) => Read::Mongo(
                mongo.clone(),
                match filter {
                    Some(f) => serde_json::from_str::<JsonValue>(f)
                        .map_err(|e| e.to_string())
                        .and_then(|json| to_document(&json).map_err(|e| e.to_string()))
                        .map_err(|e| {
                            Error::InvalidArgument(format!(
                                "filter {f:?} is not a mongodb query document, \
                                 e.g. '{{\"year\": {{\"$gt\": 2000}}}}' ({e})"
                            ))
                        })?,
                    None => doc! {},
                },
            ),
            Source::Topk(topk) => match filter {
                None => Read::Topk(topk.clone()),
                Some(_) => {
                    return Err(Error::InvalidArgument(
                        "--filter is not supported for topk:// sources".to_string(),
                    ))
                }
            },
        };
        Ok(Scan {
            name: name.to_string(),
            target: target.clone(),
            after: after.map(str::to_string),
            read,
        })
    }

    pub fn concurrency_limit(&self) -> usize {
        match self {
            Source::Duck(duck) => duck.concurrency_limit(),
            _ => usize::MAX,
        }
    }
}

pub struct Scan {
    pub name: String,
    pub target: Target,
    pub after: Option<String>,
    read: Read,
}

enum Read {
    Duck(Duckdb, Option<File>),
    Es(Es, JsonValue),
    Mongo(Mongo, BsonDoc),
    Topk(Topk),
}

impl Scan {
    /// Rows after the scan's cursor, ordered so a chunk's mark is a resume point.
    pub async fn stream(&self) -> Result<Records, Error> {
        let after = self.after.as_deref();
        match &self.read {
            Read::Duck(duck, file) => duck.stream(file.as_ref(), &self.target, after).await,
            Read::Es(es, query) => es.stream(query, &self.target, after).await,
            Read::Mongo(mongo, filter) => mongo.stream(filter, &self.target, after).await,
            Read::Topk(topk) => topk.stream(&self.target, after).await,
        }
    }
}
