use sqlx::{Executor, SqlitePool};
use topk_rs::proto::v1::data::Document;

use super::sql;

pub async fn seed(url: &str, name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
    let pool = SqlitePool::connect(&format!("{url}?mode=rwc")).await?;
    pool.execute(sql::ddl(name, &docs, &sql::SQLITE).as_str())
        .await?;
    pool.execute(sql::insert(name, &docs).as_str()).await?;
    pool.close().await;
    Ok(format!("main.{name}"))
}
