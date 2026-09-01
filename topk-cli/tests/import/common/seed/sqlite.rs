use sqlx::{Executor, SqlitePool};
use tempfile::TempDir;
use topk::import::Target;
use topk_rs::proto::v1::data::Document;

use super::sql;

pub struct Db {
    dir: TempDir,
}

impl Db {
    pub fn new() -> anyhow::Result<Db> {
        Ok(Db {
            dir: tempfile::tempdir()?,
        })
    }

    fn path(&self) -> String {
        self.dir.path().join("shop.db").display().to_string()
    }
}

#[async_trait::async_trait(?Send)]
impl super::Seed for Db {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target> {
        let pool = SqlitePool::connect(&format!("sqlite://{}?mode=rwc", self.path())).await?;
        pool.execute(sql::ddl(name, &docs, &sql::SQLITE).as_str())
            .await?;
        pool.execute(sql::insert(name, &docs).as_str()).await?;
        Ok(super::discovered(
            Target {
                from: format!("main.{name}"),
                ..Default::default()
            },
            self.url(),
        )
        .await?)
    }

    fn url(&self) -> Option<String> {
        Some(format!("sqlite://{}", self.path()))
    }
}
