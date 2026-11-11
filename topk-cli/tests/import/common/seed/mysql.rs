use sqlx::{Executor, MySqlPool};
use topk_rs::proto::v1::data::Document;

use super::sql;

pub const URL: &str = "mysql://root:root@localhost:3307/demo";

pub async fn seed(name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
    let table = format!("demo.{name}");
    let pool = MySqlPool::connect(URL).await?;
    pool.execute(sql::ddl(&table, &docs, &sql::MYSQL).as_str())
        .await?;
    pool.execute(sql::insert(&table, &docs).as_str()).await?;
    Ok(table)
}
