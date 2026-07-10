mod common;

use common::{TestScope, TwoIndices};
use elasticsearch::{http::StatusCode, MgetParts};
use serde_json::json;
use test_context::test_context;

#[test_context(TestScope)]
#[tokio::test]
async fn test_mget_ids_form_found_and_missing(scope: &TestScope) {
    scope.create().await;
    scope
        .index_docs([
            ("1", json!({ "title": "one" })),
            ("2", json!({ "title": "two" })),
        ])
        .await;

    let body = scope
        .client
        .mget(
            MgetParts::Index(&scope.name),
            json!({ "ids": ["2", "missing", "1"] }),
        )
        .await;

    let docs = body["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 3, "{body}");

    assert_eq!(docs[0]["_id"], "2");
    assert_eq!(docs[0]["found"], true);
    assert_eq!(docs[0]["_source"]["title"], "two");

    assert_eq!(docs[1]["_id"], "missing");
    assert_eq!(docs[1]["found"], false);
    assert!(docs[1].get("_source").is_none());

    assert_eq!(docs[2]["_id"], "1");
    assert_eq!(docs[2]["found"], true);
    assert_eq!(docs[2]["_source"]["title"], "one");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_mget_docs_form_with_per_doc_source(scope: &TestScope) {
    scope.create().await;
    scope
        .index_docs([("1", json!({ "title": "one", "count": 42 }))])
        .await;

    let body = scope
        .client
        .mget(
            MgetParts::Index(&scope.name),
            json!({ "docs": [
                { "_id": "1", "_source": ["title"] },
                { "_id": "1", "_source": false },
            ] }),
        )
        .await;

    let docs = body["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 2, "{body}");

    assert_eq!(docs[0]["found"], true);
    assert_eq!(docs[0]["_source"]["title"], "one");
    assert!(docs[0]["_source"].get("count").is_none());

    assert_eq!(docs[1]["found"], true);
    assert!(docs[1].get("_source").is_none());
}

#[test_context(TwoIndices)]
#[tokio::test]
async fn test_mget_root_per_doc_index(scope: &TwoIndices) {
    let idx_a = &scope.a;
    let idx_b = &scope.b;
    idx_a.create().await;
    idx_b.create().await;
    idx_a.index_docs([("1", json!({ "v": "a" }))]).await;
    idx_b.index_docs([("1", json!({ "v": "b" }))]).await;

    let client = common::Client::new();
    let body = client
        .mget(
            MgetParts::None,
            json!({ "docs": [
                { "_index": idx_b.name, "_id": "1" },
                { "_index": idx_a.name, "_id": "1" },
            ] }),
        )
        .await;

    let docs = body["docs"].as_array().unwrap();
    assert_eq!(docs.len(), 2, "{body}");
    assert_eq!(docs[0]["_index"], idx_b.name);
    assert_eq!(docs[0]["_source"]["v"], "b");
    assert_eq!(docs[1]["_index"], idx_a.name);
    assert_eq!(docs[1]["_source"]["v"], "a");
}

#[tokio::test]
async fn test_mget_root_requires_index() {
    let client = common::Client::new();
    let res = client
        .es()
        .mget(MgetParts::None)
        .body(json!({ "docs": [{ "_id": "1" }] }))
        .send()
        .await
        .expect("mget");
    assert_eq!(res.status_code(), StatusCode::BAD_REQUEST);
}
