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
async fn test_sparse_vector_rejects_non_positive_weights(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "embedding": { "type": "sparse_vector" } }))
        .await;

    let err = scope
        .search(json!({
            "query": {
                "sparse_vector": {
                    "field": "embedding",
                    "query_vector": { "0": -1.0 }
                }
            }
        }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_rejects_wrong_field_type(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "keyword" } }))
        .await;

    let err = scope
        .search(json!({
            "query": {
                "sparse_vector": {
                    "field": "title",
                    "query_vector": { "0": 1.0 }
                }
            }
        }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_accepts_prune_options(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let ids = scope
        .search_ids(json!({
            "sparse_vector": {
                "field": "embedding",
                "query_vector": { "0": 1.0 },
                "prune": true,
                "pruning_config": {
                    "tokens_freq_ratio_threshold": 5,
                    "tokens_weight_threshold": 0.4
                }
            }
        }))
        .await;
    assert_eq!(ids, vec!["1", "2"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_string_keys_return_empty(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let ids = scope
        .search_ids(json!({
            "sparse_vector": {
                "field": "embedding",
                "query_vector": { "fox": 1.0 }
            }
        }))
        .await;
    assert_eq!(ids, Vec::<&str>::new());
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_unknown_field_returns_empty(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let ids = scope
        .search_ids(json!({
            "bool": {
                "filter": [
                    { "sparse_vector": { "field": "nope", "query_vector": { "0": 1.0 } } }
                ]
            }
        }))
        .await;
    assert_eq!(ids, Vec::<&str>::new());

    let count = scope
        .count(Some(json!({
            "sparse_vector": { "field": "nope", "query_vector": { "0": 1.0 } }
        })))
        .await
        .expect("count should succeed");
    assert_eq!(count, 0);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_scores_raw_dot_product(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "sparse_vector": { "field": "embedding", "query_vector": { "0": 2.0 } }
            }
        }))
        .await
        .expect("search should succeed");

    assert!((body.score("1") - 2.0).abs() < 1e-6, "{body}");
    assert!((body.score("2") - 1.6).abs() < 1e-6, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sparse_vector_boost_scales_dot_product(scope: &TestScope) {
    setup_sparse_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "sparse_vector": {
                    "field": "embedding",
                    "query_vector": { "0": 1.0 },
                    "boost": 3.0
                }
            }
        }))
        .await
        .expect("search should succeed");

    assert!((body.score("1") - 3.0).abs() < 1e-6, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_sparse_vector_skips_negative_doc_weights(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "embedding": { "type": "sparse_vector" } }))
        .await;

    scope
        .index_docs([
            ("pos", json!({ "embedding": { "0": 1.0 } })),
            ("neg", json!({ "embedding": { "0": -1.0 } })),
        ])
        .await;

    let ids = scope
        .search_ids(json!({
            "sparse_vector": { "field": "embedding", "query_vector": { "0": 1.0 } }
        }))
        .await;

    // Diverges from ES: matching is dot > 0, so a negatively weighted dim is
    // invisible where ES matches it on dim overlap and scores it negative.
    assert_eq!(ids, vec!["pos"]);
}
