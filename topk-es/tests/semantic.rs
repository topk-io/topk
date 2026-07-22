mod common;

use common::TestScope;
use elasticsearch::http::StatusCode;
use serde_json::{json, Value};
use test_context::test_context;
use test_macros::rstest_ctx;

#[test_context(TestScope)]
#[tokio::test]
async fn test_semantic_with_sort_orders_by_field(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "n": { "type": "integer" },
            "content": { "type": "semantic_text" }
        }))
        .await;

    scope
        .index_docs([
            ("a", json!({ "n": 2, "content": "cats and kittens" })),
            ("b", json!({ "n": 1, "content": "felines everywhere" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "semantic": { "field": "content", "query": "cats" } },
            "sort": "n",
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids(), vec!["b", "a"], "{body}");
    assert_eq!(body.total(), 2, "{body}");
    assert!(body.all_scores_null());
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_with_semantic_query_combines_retrievers(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "content": { "type": "semantic_text" },
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            (
                "vec",
                json!({ "content": "bananas", "embedding": [1.0, 0.0, 0.0, 0.0] }),
            ),
            (
                "sem",
                json!({ "content": "cats and kittens", "embedding": [0.0, 1.0, 0.0, 0.0] }),
            ),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "semantic": { "field": "content", "query": "cats" } },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 1
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert!(ids.contains(&"vec".to_string()));
    assert!(ids.contains(&"sem".to_string()));
    assert_eq!(body.total(), 2, "{body}");
    assert_eq!(body.total_relation(), "gte", "{body}");
}

#[rstest_ctx(TestScope)]
#[case::must_not(
    json!({ "content": { "type": "semantic_text" } }),
    json!({ "query": { "bool": { "must_not": [{ "semantic": { "field": "content", "query": "cats" } }] } } })
)]
#[case::unindexed_field(
    json!({ "title": { "type": "text" } }),
    json!({ "query": { "semantic": { "field": "title", "query": "cats" } } })
)]
#[case::unknown_field(
    json!({ "content": { "type": "semantic_text" } }),
    json!({ "query": { "semantic": { "field": "nope", "query": "cats" } } })
)]
async fn dev_semantic_search_rejected(
    scope: &TestScope,
    #[case] properties: Value,
    #[case] query: Value,
) {
    scope.create_with_properties(properties).await;

    let err = scope.search(query).await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_semantic_query_standalone_ranks_by_similarity(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "content": { "type": "semantic_text" } }))
        .await;

    scope.index_docs([
        (
            "1",
            json!({ "content": "The cat sat lazily on the warm windowsill in the sun." }),
        ),
        (
            "2",
            json!({ "content": "Rockets use liquid oxygen and kerosene for combustion during launch." }),
        ),
    ])
    .await;

    let body = scope
        .search(json!({
            "query": { "semantic": { "field": "content", "query": "kittens and cats" } },
            "size": 2
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["1", "2"],
        "most semantically similar document should rank first: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_semantic_multi_clause_scores_combine(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "content": { "type": "semantic_text" } }))
        .await;

    scope.index_docs([
        (
            "1",
            json!({ "content": "The cat sat lazily on the warm windowsill in the sun." }),
        ),
        (
            "2",
            json!({ "content": "Rockets use liquid oxygen and kerosene for combustion during launch." }),
        ),
    ])
    .await;

    let body = scope
        .search(json!({
            "query": { "bool": { "should": [
                { "semantic": { "field": "content", "query": "kittens and cats" } },
                { "semantic": { "field": "content", "query": "sleepy pets indoors" } }
            ] } },
            "size": 2
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["1", "2"],
        "both semantic clauses should score in one query and favor the cat doc: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_semantic_query_nested_in_bool_filter(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "category": { "type": "keyword" },
            "content": { "type": "semantic_text" }
        }))
        .await;

    scope.index_docs([
        (
            "1",
            json!({ "category": "a", "content": "Kittens love chasing toy mice around the living room floor." }),
        ),
        (
            "2",
            json!({ "category": "b", "content": "Rockets use liquid oxygen and kerosene for combustion during launch." }),
        ),
        (
            "3",
            json!({ "category": "b", "content": "Kittens love chasing toy mice around the living room floor." }),
        ),
    ])
    .await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "filter": [{ "term": { "category": "b" } }],
                    "must": [{ "semantic": { "field": "content", "query": "kittens and cats" } }]
                }
            }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["3", "2"],
        "bool.filter should exclude category \"a\", and the remaining docs should still rank by similarity: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_semantic_empty_query_rejected(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "content": { "type": "semantic_text" } }))
        .await;
    scope
        .index_docs([("1", json!({ "content": "cats sleep on warm windowsills" }))])
        .await;

    let err = scope
        .search(json!({ "query": { "semantic": { "field": "content", "query": "" } } }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}
