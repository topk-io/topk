mod common;

use common::TestScope;
use elasticsearch::http::StatusCode;
use serde_json::json;
use test_context::test_context;

async fn setup_sparse_docs(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "sparse_vector" },
            "title": { "type": "keyword" }
        }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "embedding": { "0": 1.0 }, "title": "a" })),
            (
                "2",
                json!({ "embedding": { "0": 0.8, "1": 0.6 }, "title": "b" }),
            ),
            ("3", json!({ "embedding": { "1": 1.0 }, "title": "c" })),
            ("4", json!({ "title": "d" })),
        ])
        .await;
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_ranks_by_dot_product(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "sparse_vector": { "field": "embedding", "query_vector": { "0": 1.0 } }
            }
        }))
        .await
        .expect("search should succeed");

    let mut ids = body.hit_ids();
    ids.truncate(2);
    assert_eq!(
        ids,
        vec!["1", "2"],
        "only overlapping dims match, ranked by dot product: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_filter_matches_overlapping_dims(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let ids = scope
        .search_ids(json!({
            "bool": {
                "filter": [
                    { "sparse_vector": { "field": "embedding", "query_vector": { "0": 1.0 } } }
                ]
            }
        }))
        .await;
    assert_eq!(ids, vec!["1", "2"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_must_not_matches_complement(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let ids = scope
        .search_ids(json!({
            "bool": {
                "must_not": [
                    { "sparse_vector": { "field": "embedding", "query_vector": { "0": 1.0 } } }
                ]
            }
        }))
        .await;
    assert_eq!(ids, vec!["3", "4"], "zero-dot and missing-field docs match");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_count_with_filter(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let count = scope
        .count(Some(json!({
            "sparse_vector": { "field": "embedding", "query_vector": { "0": 1.0 } }
        })))
        .await
        .expect("count should succeed");
    assert_eq!(count, 2);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_filter_with_bm25_scoring(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let ids = scope
        .search_ids(json!({
            "bool": {
                "must": [{ "term": { "title": "b" } }],
                "filter": [
                    { "sparse_vector": { "field": "embedding", "query_vector": { "0": 1.0 } } }
                ]
            }
        }))
        .await;
    assert_eq!(ids, vec!["2"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_sparse_vector_rejects_string_keys(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "embedding": { "type": "sparse_vector" } }))
        .await;

    let err = scope
        .search(json!({
            "query": {
                "sparse_vector": {
                    "field": "embedding",
                    // Diverges from ES: no vocabulary tokens, integer keys only.
                    "query_vector": { "fox": 1.0 }
                }
            }
        }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_sparse_vector_unknown_field_rejected(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "embedding": { "type": "sparse_vector" } }))
        .await;

    let err = scope
        .search(json!({
            "query": {
                "bool": {
                    "filter": [
                        { "sparse_vector": { "field": "nope", "query_vector": { "0": 1.0 } } }
                    ]
                }
            }
        }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}
