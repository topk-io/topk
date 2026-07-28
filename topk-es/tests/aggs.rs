mod common;

use common::{BooksContext, TestScope};
use serde_json::{json, Value};
use test_context::test_context;
use test_macros::rstest_ctx;

#[test_context(BooksContext)]
#[tokio::test]
async fn test_terms_agg_with_avg_sub_agg(books: &BooksContext) {
    let resp = books
        .search(json!({
            "size": 0,
            "aggs": {
                "by_genre": {
                    "terms": { "field": "genre" },
                    "aggs": { "avg_rating": { "avg": { "field": "rating" } } }
                }
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(resp.hit_ids().len(), 0, "size:0 should return no hits");

    let buckets = resp.buckets("by_genre");
    assert_eq!(buckets.len(), 5, "5 distinct genres: {buckets:?}");

    let fiction = buckets
        .iter()
        .find(|b| b["key"] == "fiction")
        .expect("fiction bucket");
    assert_eq!(fiction["doc_count"], 4);
    let avg_rating = fiction["avg_rating"]["value"].as_f64().unwrap();
    assert!((avg_rating - 3.975).abs() < 1e-6);
}

#[test_context(BooksContext)]
#[tokio::test]
async fn test_bare_metric_agg_with_size_zero(books: &BooksContext) {
    let resp = books
        .search(json!({
            "size": 0,
            "aggs": { "avg_rating": { "avg": { "field": "rating" } } }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(resp.hit_ids().len(), 0);
    assert_eq!(resp.total(), 10);

    let value = resp.agg_value("avg_rating");
    assert!((value - 4.12).abs() < 1e-6);
}

#[test_context(BooksContext)]
#[tokio::test]
async fn test_multiple_sibling_top_level_aggs(books: &BooksContext) {
    let resp = books
        .search(json!({
            "size": 0,
            "aggs": {
                "avg_rating": { "avg": { "field": "rating" } },
                "min_year": { "min": { "field": "published_year" } }
            }
        }))
        .await
        .expect("search should succeed");

    assert!(resp.agg("avg_rating")["value"].is_number(), "{resp}");
    assert_eq!(
        resp.agg_value("min_year"),
        1813.0,
        "pride (1813) is the oldest book"
    );
}

#[rstest_ctx(TestScope)]
// ES scopes aggs to the `k` docs retrieved by a top-level `knn`; TopK scopes them
// over the full match set.
#[case::dev_knn(
    json!({
        "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" },
        "n": { "type": "integer" }
    }),
    vec![
        ("1", json!({ "embedding": [1.0, 0.0], "n": 1 })),
        ("2", json!({ "embedding": [0.9, 0.1], "n": 2 })),
        ("3", json!({ "embedding": [0.0, 1.0], "n": 3 })),
    ],
    json!({
        "knn": { "field": "embedding", "query_vector": [1.0, 0.0], "k": 2 },
        "size": 2,
        "aggs": { "hit_count": { "value_count": { "field": "n" } } }
    }),
    2,
    3
)]
#[case::semantic(
    json!({
        "content": { "type": "semantic_text" },
        "n": { "type": "integer" }
    }),
    vec![
        (
            "1",
            json!({ "content": "The cat sat lazily on the warm windowsill in the sun.", "n": 1 }),
        ),
        (
            "2",
            json!({ "content": "Rockets use liquid oxygen and kerosene for combustion during launch.", "n": 2 }),
        ),
    ],
    json!({
        "query": { "semantic": { "field": "content", "query": "kittens and cats" } },
        "size": 1,
        "aggs": { "hit_count": { "value_count": { "field": "n" } } }
    }),
    1,
    2
)]
async fn test_aggs_scope_over_match_set_not_retrieved_hits(
    scope: &TestScope,
    #[case] properties: Value,
    #[case] docs: Vec<(&str, Value)>,
    #[case] search: Value,
    #[case] expected_hits: u64,
    #[case] expected_agg: u64,
) {
    scope.create_with_properties(properties).await;
    scope.index_docs(docs).await;

    let resp = scope.search(search).await.expect("search should succeed");

    assert_eq!(resp.hit_ids().len(), expected_hits as usize, "{resp}");
    assert_eq!(
        resp.agg_value("hit_count"),
        expected_agg as f64,
        "aggs scope over the query match set, not the retrieved hits: {resp}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_metric_aggs_over_empty_match(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "g": { "type": "keyword" }, "n": { "type": "integer" } }))
        .await;
    scope.index_docs([("1", json!({ "g": "a", "n": 1 }))]).await;

    let body = scope
        .search(json!({
            "query": { "term": { "g": "zzz" } },
            "aggs": {
                "s": { "sum": { "field": "n" } },
                "c": { "value_count": { "field": "n" } },
                "a": { "avg": { "field": "n" } },
                "mn": { "min": { "field": "n" } }
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.agg("s")["value"], 0.0, "{body}");
    assert_eq!(body.agg("c")["value"], 0.0, "{body}");
    assert!(body.agg("a")["value"].is_null(), "{body}");
    assert!(body.agg("mn")["value"].is_null(), "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_terms_agg_breaks_doc_count_ties_by_key(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "g": { "type": "keyword" } }))
        .await;
    scope
        .index_docs([
            ("1", json!({ "g": "k3" })),
            ("2", json!({ "g": "k1" })),
            ("3", json!({ "g": "k2" })),
        ])
        .await;

    let body = scope
        .search(json!({ "size": 0, "aggs": { "t": { "terms": { "field": "g" } } } }))
        .await
        .expect("search should succeed");

    let keys: Vec<&str> = body.agg("t")["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, vec!["k1", "k2", "k3"], "{body}");
}

// The engine applies `size` before we can break `doc_count` ties by key, so a tie
// straddling the limit keeps whichever buckets the engine happened to return —
// here `k1`/`k3`, where ES orders by key first and keeps `k1`/`k2`. Only the
// selection diverges; what comes back is still key-ordered.
#[test_context(TestScope)]
#[tokio::test]
async fn bug_terms_agg_tie_break_applied_after_size_limit(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "g": { "type": "keyword" } }))
        .await;
    scope
        .index_docs([
            ("1", json!({ "g": "k3" })),
            ("2", json!({ "g": "k1" })),
            ("3", json!({ "g": "k2" })),
        ])
        .await;

    let body = scope
        .search(json!({ "size": 0, "aggs": { "t": { "terms": { "field": "g", "size": 2 } } } }))
        .await
        .expect("search should succeed");

    let keys: Vec<&str> = body
        .buckets("t")
        .iter()
        .map(|b| b["key"].as_str().unwrap())
        .collect();
    assert_eq!(keys, vec!["k1", "k3"], "{body}");
}
