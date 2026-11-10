use sqlx::{Executor, MySqlPool};
use topk_rs::proto::v1::data::Document;

use super::sql;

pub struct MySql;

impl MySql {
    pub const URL: &str = "mysql://root:root@localhost:3307/demo";

    pub fn new() -> anyhow::Result<MySql> {
        Ok(MySql)
    }
}

#[async_trait::async_trait(?Send)]
impl super::Seed for MySql {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
        let table = format!("demo.{name}");
        let pool = MySqlPool::connect(Self::URL).await?;
        pool.execute(sql::ddl(&table, &docs, &sql::MYSQL).as_str())
            .await?;
        pool.execute(sql::insert(&table, &docs).as_str()).await?;
        Ok(table)
    }

    fn url(&self) -> Option<String> {
        Some(Self::URL.to_string())
    }
}
