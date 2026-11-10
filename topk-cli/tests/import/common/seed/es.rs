use elasticsearch::http::request::JsonBody;
use elasticsearch::http::transport::Transport;
use elasticsearch::indices::IndicesDeleteParts;
use elasticsearch::params::Refresh;
use elasticsearch::{BulkParts, Elasticsearch};
use serde_json::json;
use topk_rs::proto::v1::data::Document;

use crate::import::common::json as doc_json;

pub async fn seed(name: &str, docs: Vec<Document>) -> anyhow::Result<String> {
    let client = Elasticsearch::new(Transport::single_node("http://localhost:19200")?);
    let _ = client
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

    client
        .bulk(BulkParts::Index(name))
        .body(body)
        .refresh(Refresh::True)
        .send()
        .await?
        .error_for_status_code()?;

    Ok(name.to_string())
}
