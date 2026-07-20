mod common;

use common::{BooksContext, TestScope};
use ddb_test_macros::rstest_ctx;
use serde_json::{json, Value};
use test_context::test_context;

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
async fn bug_bare_metric_agg_with_size_zero(books: &BooksContext) {
    let resp = books
        .search(json!({
            "size": 0,
            "aggs": { "avg_rating": { "avg": { "field": "rating" } } }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(resp.hit_ids().len(), 0);
    assert_eq!(resp.total(), 0);

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
#[case::knn(
    json!({
        "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" }
    }),
    vec![
        ("1", json!({ "embedding": [1.0, 0.0] })),
        ("2", json!({ "embedding": [0.9, 0.1] })),
        ("3", json!({ "embedding": [0.0, 1.0] })),
    ],
    json!({
        "knn": { "field": "embedding", "query_vector": [1.0, 0.0], "k": 2 },
        "size": 2,
        "aggs": { "hit_count": { "value_count": { "field": "embedding" } } }
    }),
    2,
    3
)]
#[case::semantic(
    json!({ "content": { "type": "semantic_text" } }),
    vec![
        (
            "1",
            json!({ "content": "The cat sat lazily on the warm windowsill in the sun." }),
        ),
        (
            "2",
            json!({ "content": "Rockets use liquid oxygen and kerosene for combustion during launch." }),
        ),
    ],
    json!({
        "query": { "semantic": { "field": "content", "query": "kittens and cats" } },
        "size": 1,
        "aggs": { "hit_count": { "value_count": { "field": "content" } } }
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
