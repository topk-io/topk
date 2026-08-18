use std::ops::Deref;

use async_trait::async_trait;
use mongodb::bson;
use topk::import::Target;
use topk_rs::proto::v1::data::Document;

use crate::common::json;

pub struct Mongo(mongodb::Client);

impl Deref for Mongo {
    type Target = mongodb::Client;

    fn deref(&self) -> &mongodb::Client {
        &self.0
    }
}

impl Mongo {
    pub const URL: &str = "mongodb://localhost:27017/demo";

    // Parsing a non-srv mongodb:// URI does no I/O, so block_on is safe in a
    // sync rstest case expression.
    pub fn client() -> Mongo {
        Mongo(
            futures::executor::block_on(mongodb::Client::with_uri_str(Self::URL))
                .expect("mongo client"),
        )
    }
}

#[async_trait(?Send)]
impl super::Seed for Mongo {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target> {
        let docs: Vec<bson::Document> = docs
            .iter()
            .map(|doc| bson::to_document(&serde_json::Value::Object(json(doc))))
            .collect::<Result<_, _>>()?;

        self.database("demo")
            .collection::<bson::Document>(name)
            .insert_many(docs)
            .await?;

        Ok(super::discovered(
            Target {
                from: name.to_string(),
                ..Default::default()
            },
            self.url(),
        )
        .await?)
    }

    fn url(&self) -> Option<String> {
        Some(Self::URL.to_string())
    }
}
