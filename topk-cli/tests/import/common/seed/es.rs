use std::ops::Deref;

use async_trait::async_trait;
use elasticsearch::http::request::JsonBody;
use elasticsearch::http::transport::Transport;
use elasticsearch::indices::IndicesDeleteParts;
use elasticsearch::params::Refresh;
use elasticsearch::{BulkParts, Elasticsearch};
use serde_json::json;
use topk::import::Target;
use topk_rs::proto::v1::data::Document;

use crate::common::json as doc_json;

pub struct Es(Elasticsearch);

impl Deref for Es {
    type Target = Elasticsearch;

    fn deref(&self) -> &Elasticsearch {
        &self.0
    }
}

impl Es {
    pub const URL: &str = "http://localhost:19200";

    pub fn client() -> Es {
        Es(Elasticsearch::new(
            Transport::single_node(Self::URL).expect("es transport"),
        ))
    }
}

#[async_trait(?Send)]
impl super::Seed for Es {
    async fn seed(&self, name: &str, docs: Vec<Document>) -> anyhow::Result<Target> {
        let _ = self
            .indices()
            .delete(IndicesDeleteParts::Index(&[name]))
            .send()
            .await;

        let mut body: Vec<JsonBody<serde_json::Value>> = Vec::new();
        for doc in &docs {
            // `_id` is ES metadata, not a source field.
            let mut fields = doc_json(doc);
            let id = fields.remove("_id").expect("_id");
            body.push(json!({ "index": { "_id": id } }).into());
            body.push(serde_json::Value::Object(fields).into());
        }

        self.bulk(BulkParts::Index(name))
            .body(body)
            .refresh(Refresh::True)
            .send()
            .await?
            .error_for_status_code()?;

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
        // The import scheme; Self::URL stays http:// for the raw seeding client.
        Some("es://localhost:19200".to_string())
    }
}
