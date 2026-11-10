pub mod es;
pub mod mongo;
pub mod mysql;
pub mod pg;
pub mod rustfs;
pub mod sql;
pub mod sqlite;

use topk_rs::proto::v1::data::Document;

use super::Scratch;

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
}

pub struct Seeded {
    pub from: String,
    pub url: String,
}

impl Scratch {
    pub async fn seed(&self, backend: Backend, name: &str, docs: Vec<Document>) -> Seeded {
        let remote = self.name(name);
        let (from, url) = match backend {
            Backend::Sqlite => {
                let url = format!("sqlite://{}", self.path(&format!("{name}.db")).display());
                let from = sqlite::seed(&url, name, docs).await.unwrap();
                (from, url)
            }
            Backend::Postgres => (
                pg::Pg::seed(&remote, docs).await.unwrap(),
                pg::Pg::URL.into(),
            ),
            Backend::Mysql => (mysql::seed(&remote, docs).await.unwrap(), mysql::URL.into()),
            Backend::Mongo => (mongo::seed(&remote, docs).await.unwrap(), mongo::URL.into()),
            Backend::Elasticsearch => (
                es::seed(&remote, docs).await.unwrap(),
                "es://localhost:19200".into(),
            ),
            Backend::Parquet => {
                let path = self.seed_parquet(name, docs);
                (path.clone(), path)
            }
            Backend::S3 => {
                let input = self.jsonl(&format!("{name}.jsonl"), &docs);
                let path = rustfs::seed(&remote, &input).unwrap();
                (path.clone(), path)
            }
        };
        Seeded { from, url }
    }
}
