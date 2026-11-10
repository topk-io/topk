pub mod es;
pub mod mongo;
pub mod mysql;
pub mod parquet;
pub mod pg;
pub mod rustfs;
pub mod sql;
pub mod sqlite;

use async_trait::async_trait;

use topk_rs::proto::v1::data::Document;

use super::{topk, unique_name, TestCommand};

#[derive(Clone, Copy, Debug)]
pub enum Backend {
    Sqlite,
    Postgres,
    Mysql,
    Mongo,
    Elasticsearch,
    Parquet,
    S3,
}

impl Backend {
    /// "published after 1950", in the language this source filters in.
    pub fn filter(self) -> &'static str {
        match self {
            Backend::Mongo => r#"{"published_year": {"$gt": 1950}}"#,
            Backend::Elasticsearch => r#"{"range": {"published_year": {"gt": 1950}}}"#,
            _ => "published_year > 1950",
        }
    }

    /// The type a boolean column is discovered as; sqlite type names are
    /// affinity hints, so its BOOLEAN surfaces as int.
    pub fn bool_type(self) -> &'static str {
        match self {
            Backend::Sqlite => "int",
            _ => "bool",
        }
    }

    pub async fn seed(self, docs: Vec<Document>) -> Seeded {
        let seed: Box<dyn Seed + Send> = match self {
            Backend::Sqlite => Box::new(sqlite::Db::new().unwrap()),
            Backend::Postgres => Box::new(pg::Pg::new().unwrap()),
            Backend::Mysql => Box::new(mysql::MySql::new().unwrap()),
            Backend::Mongo => Box::new(mongo::Mongo::client()),
            Backend::Elasticsearch => Box::new(es::Es::client()),
            Backend::Parquet => Box::new(parquet::File::new().unwrap()),
            Backend::S3 => Box::new(rustfs::S3::new().unwrap()),
        };
        let from = seed
            .seed(&unique_name("books"), docs)
            .await
            .unwrap_or_else(|e| panic!("seed {self:?}: {e:#}"));
        Seeded {
            from,
            url: seed.url(),
            env: match self {
                Backend::S3 => rustfs::S3::ENV,
                _ => &[],
            },
            _keep: seed,
        }
    }
}

/// A seeded object, and how the CLI reaches it.
pub struct Seeded {
    pub from: String,
    pub url: Option<String>,
    pub env: &'static [(&'static str, &'static str)],
    /// Owns the temporary directory or connection behind the object.
    pub _keep: Box<dyn Seed + Send>,
}

impl Seeded {
    /// `topk import [<url>]`, with the environment the source needs.
    pub fn import(&self) -> TestCommand {
        topk()
            .envs(self.env)
            .args(std::iter::once("import").chain(self.url.as_deref()))
    }
}

#[async_trait(?Send)]
pub trait Seed {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<String>;

    /// Where to connect; `None` when `from` is the whole locator, as for files.
    fn url(&self) -> Option<String> {
        None
    }
}
