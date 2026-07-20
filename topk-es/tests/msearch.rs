mod common;

use common::{hit_ids, TestScope, TwoIndices};
use elasticsearch::MsearchParts;
use serde_json::json;
use test_context::test_context;

#[test_context(TestScope)]
#[tokio::test]
async fn test_msearch_multiple_searches_in_order(scope: &TestScope) {
    scope.create().await;
    scope
        .index_docs([
            ("1", json!({ "title": "alpha", "count": 1 })),
            ("2", json!({ "title": "beta", "count": 2 })),
        ])
        .await;

    let body = scope
        .client
        .msearch(
            MsearchParts::Index(&[scope.name.as_str()]),
            vec![
                json!({}),
                json!({ "query": { "term": { "count": 1 } } }),
                json!({}),
                json!({ "query": { "term": { "count": 2 } } }),
            ],
        )
        .await;

    let responses = body["responses"].as_array().unwrap();
    assert_eq!(responses.len(), 2, "{body}");
    assert_eq!(responses[0]["status"], 200, "{body}");
    assert_eq!(hit_ids(&responses[0]), vec!["1"]);
    assert_eq!(responses[1]["status"], 200, "{body}");
    assert_eq!(hit_ids(&responses[1]), vec!["2"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_msearch_isolates_per_item_error(scope: &TestScope) {
    scope.create().await;
    scope.index_docs([("1", json!({ "title": "alpha" }))]).await;

    let client = common::Client::new();
    let body = client
        .msearch(
            MsearchParts::None,
            vec![
                json!({ "index": scope.name }),
                json!({ "query": { "match_all": {} } }),
                json!({ "index": "missingindex" }),
                json!({ "query": { "match_all": {} } }),
            ],
        )
        .await;

    let responses = body["responses"].as_array().unwrap();
    assert_eq!(responses.len(), 2, "{body}");

    assert_eq!(responses[0]["status"], 200, "{body}");
    assert_eq!(hit_ids(&responses[0]), vec!["1"]);

    assert_eq!(responses[1]["status"], 404, "{body}");
    assert!(responses[1].get("error").is_some());
    assert!(responses[1].get("hits").is_none());
}

#[test_context(TwoIndices)]
#[tokio::test]
async fn test_msearch_root_per_header_index(scope: &TwoIndices) {
    let idx_a = &scope.a;
    let idx_b = &scope.b;
    idx_a.create().await;
    idx_b.create().await;
    idx_a.index_docs([("1", json!({ "v": "a" }))]).await;
    idx_b.index_docs([("1", json!({ "v": "b" }))]).await;

    let client = common::Client::new();
    let body = client
        .msearch(
            MsearchParts::None,
            vec![
                json!({ "index": idx_b.name }),
                json!({ "query": { "match_all": {} } }),
                json!({ "index": idx_a.name }),
                json!({ "query": { "match_all": {} } }),
            ],
        )
        .await;

    let responses = body["responses"].as_array().unwrap();
    assert_eq!(responses.len(), 2, "{body}");
    assert_eq!(
        responses[0]["hits"]["hits"][0]["_index"], idx_b.name,
        "{body}"
    );
    assert_eq!(responses[0]["hits"]["hits"][0]["_source"]["v"], "b");
    assert_eq!(
        responses[1]["hits"]["hits"][0]["_index"], idx_a.name,
        "{body}"
    );
    assert_eq!(responses[1]["hits"]["hits"][0]["_source"]["v"], "a");
}
