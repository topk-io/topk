use mongodb::bson;
use topk_rs::proto::v1::data::Document;

use crate::import::common::json;

pub const URL: &str = "mongodb://localhost:27017/demo";

pub async fn client() -> mongodb::Client {
    mongodb::Client::with_uri_str(URL).await.unwrap()
}

pub async fn seed(name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
    let docs: Vec<bson::Document> = docs
        .iter()
        .map(|doc| bson::to_document(&serde_json::Value::Object(json(doc))))
        .collect::<Result<_, _>>()?;

    client()
        .await
        .database("demo")
        .collection::<bson::Document>(name)
        .insert_many(docs)
        .await?;

    Ok(name.to_string())
}
