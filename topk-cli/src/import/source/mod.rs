pub mod codec;
mod duck;
mod es;
mod mongo;
pub mod uri;

use futures::stream::BoxStream;
use topk_rs::proto::v1::data::Value;

use crate::import::error::Error;
use crate::import::source::uri::Uri;
use crate::import::spec::{Field, Target};

pub use duck::Duckdb;

use es::Es;
use mongo::Mongo;

pub type Record = Vec<(String, Value)>;
pub type Records = BoxStream<'static, Result<Record, Error>>;

pub enum Source {
    Duck(Duckdb),
    Es(Es),
    Mongo(Mongo),
}

impl Source {
    pub async fn catalog(&self, uri: &Uri) -> Result<Vec<Table>, Error> {
        match self {
            Source::Duck(duck) => duck.catalog(uri).await,
            Source::Es(es) => es.catalog().await,
            Source::Mongo(mongo) => mongo.catalog().await,
        }
    }

    pub async fn stream(&self, target: &Target) -> Result<Records, Error> {
        match self {
            Source::Duck(duck) => duck.stream(target).await,
            Source::Es(es) => es.stream(target).await,
            Source::Mongo(mongo) => mongo.stream(target).await,
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
        Uri::File { .. } => Source::Duck(Duckdb::Files),
    })
}
