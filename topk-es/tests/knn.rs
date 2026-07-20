mod common;

use common::{BooksContext, TestScope};
use test_macros::rstest_ctx;
use elasticsearch::http::StatusCode;
use serde_json::{json, Value};
use test_context::test_context;

async fn setup_hybrid_docs(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "body": { "type": "text" },
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            (
                "both",
                json!({ "body": "cats", "embedding": [1.0, 0.0, 0.0, 0.0] }),
            ),
            (
                "vector",
                json!({ "body": "dogs", "embedding": [0.9, 0.1, 0.0, 0.0] }),
            ),
            (
                "text",
                json!({ "body": "cats", "embedding": [0.0, 1.0, 0.0, 0.0] }),
            ),
        ])
        .await;
}

#[rstest_ctx(TestScope)]
#[case::cosine(
    "cosine",
    vec![
        ("1", json!({ "embedding": [1.0, 0.0, 0.0, 0.0] })),
        ("2", json!({ "embedding": [0.9, 0.1, 0.0, 0.0] })),
        ("3", json!({ "embedding": [0.0, 1.0, 0.0, 0.0] })),
        ("4", json!({ "embedding": [0.0, 0.0, 1.0, 0.0] })),
    ],
    vec!["1", "2"]
)]
// `dot_product` requires unit-length vectors, so rank by direction, not magnitude.
#[case::dot_product(
    "dot_product",
    vec![
        ("1", json!({ "embedding": [1.0, 0.0, 0.0, 0.0] })),
        ("2", json!({ "embedding": [0.8, 0.6, 0.0, 0.0] })),
        ("3", json!({ "embedding": [0.0, 1.0, 0.0, 0.0] })),
    ],
    vec!["1", "2"]
)]
async fn test_knn_ranks_by_similarity_metric(
    scope: &TestScope,
    #[case] similarity: &str,
    #[case] docs: Vec<(&str, Value)>,
    #[case] expected: Vec<&str>,
) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": similarity }
        }))
        .await;

    scope.index_docs(docs).await;

    let body = scope
        .search(json!({
            "knn": { "field": "embedding", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2 }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), expected, "nearest by {similarity}: {body}");
}

#[rstest_ctx(TestScope)]
#[case::dot_product_non_unit("dot_product", json!([2.0, 0.0, 0.0, 0.0]))]
#[case::cosine_zero_magnitude("cosine", json!([0.0, 0.0, 0.0, 0.0]))]
async fn test_indexing_rejects_invalid_vector(
    scope: &TestScope,
    #[case] similarity: &str,
    #[case] embedding: Value,
) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": similarity }
        }))
        .await;

    let res = scope
        .index_doc("1", json!({ "embedding": embedding }))
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "{res}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_cosine_score_reflects_similarity(scope: &TestScope) {
    scope
        .create_with_properties(
            json!({ "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" } }),
        )
        .await;

    scope
        .index_docs([
            ("same", json!({ "embedding": [1.0, 0.0, 0.0, 0.0] })),
            ("orthogonal", json!({ "embedding": [0.0, 1.0, 0.0, 0.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": { "field": "embedding", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2 }
        }))
        .await
        .expect("search should succeed");
    // ES normalises cosine into [0, 1] as (1 + cos) / 2: identical vectors score
    // 1.0 and orthogonal ones 0.5, not the raw cosine of 0.0.
    assert!((body.score("same") - 1.0).abs() < 1e-6);
    assert!((body.score("orthogonal") - 0.5).abs() < 1e-6);
    assert!((body.max_score() - body.score("same")).abs() < 1e-6);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_dot_product_score_is_es_normalized(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "dot_product" }
        }))
        .await;

    scope
        .index_docs([
            ("same", json!({ "embedding": [1.0, 0.0, 0.0, 0.0] })),
            ("orthogonal", json!({ "embedding": [0.0, 1.0, 0.0, 0.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": { "field": "embedding", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2 }
        }))
        .await
        .expect("search should succeed");

    // ES normalises dot_product into [0, 1] as (1 + dot) / 2, so an orthogonal
    // unit vector (dot = 0) reports 0.5 rather than the raw 0.0.
    assert!((body.score("same") - 1.0).abs() < 1e-6, "{body}");
    assert!((body.score("orthogonal") - 0.5).abs() < 1e-6, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_euclidean_score_reflects_distance(scope: &TestScope) {
    scope
        .create_with_properties(
            json!({ "embedding": { "type": "dense_vector", "dims": 4, "similarity": "l2_norm" } }),
        )
        .await;

    scope
        .index_docs([
            ("1", json!({ "embedding": [1.0, 0.0, 0.0, 0.0] })),
            ("2", json!({ "embedding": [10.0, 0.0, 0.0, 0.0] })),
            ("3", json!({ "embedding": [100.0, 0.0, 0.0, 0.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": { "field": "embedding", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2 }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(
        body.hit_ids(),
        vec!["1", "2"],
        "k:2 should return the two nearest by l2_norm: {body}"
    );
    let score_1 = body.score("1");
    let score_2 = body.score("2");
    assert!((score_1 - 1.0).abs() < 1e-6);
    assert!(score_2 > 0.0 && score_2 < score_1);
    assert!((body.max_score() - score_1).abs() < 1e-6);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_with_filter(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "category": { "type": "keyword" },
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            (
                "1",
                json!({ "category": "a", "embedding": [1.0, 0.0, 0.0, 0.0] }),
            ),
            (
                "2",
                json!({ "category": "b", "embedding": [0.9, 0.1, 0.0, 0.0] }),
            ),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 5,
                "filter": { "term": { "category": "b" } }
            }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["2"],
        "filter should restrict candidates before ranking: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_filter_array_combines_filters(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "category": { "type": "keyword" },
            "n": { "type": "integer" },
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            (
                "1",
                json!({ "category": "a", "n": 2, "embedding": [1.0, 0.0, 0.0, 0.0] }),
            ),
            (
                "2",
                json!({ "category": "b", "n": 3, "embedding": [0.9, 0.1, 0.0, 0.0] }),
            ),
            (
                "3",
                json!({ "category": "b", "n": 1, "embedding": [0.8, 0.2, 0.0, 0.0] }),
            ),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 5,
                "filter": [
                    { "term": { "category": "b" } },
                    { "range": { "n": { "gte": 2 } } }
                ]
            }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["2"],
        "knn.filter array should AND the filters before ranking: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_array_form_combines_vector_queries(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            ("x", json!({ "embedding": [1.0, 0.0, 0.0, 0.0] })),
            ("y", json!({ "embedding": [0.0, 1.0, 0.0, 0.0] })),
            ("mid", json!({ "embedding": [0.5, 0.5, 0.0, 0.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": [
                {
                    "field": "embedding",
                    "query_vector": [1.0, 0.0, 0.0, 0.0],
                    "k": 1,
                    "num_candidates": 3
                },
                {
                    "field": "embedding",
                    "query_vector": [0.0, 1.0, 0.0, 0.0],
                    "k": 1,
                    "num_candidates": 3
                }
            ],
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert_eq!(ids.len(), 2, "{body}");
    assert!(ids.contains(&"x".to_string()));
    assert!(ids.contains(&"y".to_string()));
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_knn_query_rrf_unions_retrievers_without_phantom_hits(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "body": { "type": "text" },
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    // The index is larger than what the retrievers should surface: only "text"
    // matches the term, and knn k:2 returns the two nearest vectors. The four
    // "noise" docs match neither and must not leak into the fused results.
    scope
        .index_docs([
            (
                "text",
                json!({ "body": "cats", "embedding": [0.0, 0.0, 0.0, 1.0] }),
            ),
            (
                "vec1",
                json!({ "body": "dogs", "embedding": [1.0, 0.0, 0.0, 0.0] }),
            ),
            (
                "vec2",
                json!({ "body": "dogs", "embedding": [0.9, 0.1, 0.0, 0.0] }),
            ),
            (
                "noise1",
                json!({ "body": "dogs", "embedding": [0.0, 1.0, 0.0, 0.0] }),
            ),
            (
                "noise2",
                json!({ "body": "dogs", "embedding": [0.0, 0.0, 1.0, 0.0] }),
            ),
            (
                "noise3",
                json!({ "body": "dogs", "embedding": [0.0, 0.7, 0.7, 0.0] }),
            ),
            (
                "noise4",
                json!({ "body": "dogs", "embedding": [0.5, 0.0, 0.5, 0.0] }),
            ),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 2,
                "num_candidates": 10
            },
            "rank": { "rrf": { "rank_window_size": 10 } },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert_eq!(
        body.total(),
        3,
        "fusion should union the retrievers (1 term + 2 knn), not the whole index: {body}"
    );
    assert!(ids.contains(&"text".to_string()), "{body}");
    assert!(ids.contains(&"vec1".to_string()), "{body}");
    assert!(ids.contains(&"vec2".to_string()), "{body}");
    for noise in ["noise1", "noise2", "noise3", "noise4"] {
        assert!(
            !ids.contains(&noise.to_string()),
            "unrelated doc {noise} received a phantom RRF score: {body}"
        );
    }
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_knn_query_rrf_combines_lexical_and_vector_results(scope: &TestScope) {
    setup_hybrid_docs(scope).await;

    let body = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 2,
                "num_candidates": 3
            },
            "rank": { "rrf": {} },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert_eq!(ids.len(), 3, "{body}");
    assert!(ids.contains(&"both".to_string()));
    assert!(ids.contains(&"vector".to_string()));
    assert!(ids.contains(&"text".to_string()));
    assert_eq!(
        ids[0], "both",
        "top-ranked in both the text and vector retrievers should fuse to rank first: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_over_byte_vectors_ranks_by_similarity(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 2, "element_type": "byte", "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            ("close", json!({ "embedding": [127, 1] })),
            ("far", json!({ "embedding": [-128, 127] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [127, 0],
                "k": 2
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids(), vec!["close", "far"], "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_over_bit_vectors_ranks_by_hamming(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 16, "element_type": "bit" }
        }))
        .await;

    // Bit vectors pack 8 bits per signed byte: [-1, 0] is 0xFF00, [0, -1] is 0x00FF.
    scope
        .index_docs([
            ("close", json!({ "embedding": [-1, 0] })),
            ("far", json!({ "embedding": [0, -1] })),
        ])
        .await;

    // Query 0xFF01 differs from "close" by 1 bit and from "far" by 15 bits.
    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [-1, 1],
                "k": 2
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids(), vec!["close", "far"], "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_knn_byte_vector_rejects_fractional_query(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 2, "element_type": "byte", "similarity": "cosine" }
        }))
        .await;

    let err = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [0.5, 1.0],
                "k": 1
            }
        }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_similarity_cutoff_filters_low_similarity_hits(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            ("close", json!({ "embedding": [1.0, 0.0] })),
            ("far", json!({ "embedding": [0.0, 1.0] })),
        ])
        .await;

    let unfiltered = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0],
                "k": 2
            }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(unfiltered.hit_ids().len(), 2, "{unfiltered}");

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0],
                "k": 2,
                "similarity": 0.5
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(
        body.hit_ids(),
        vec!["close"],
        "similarity cutoff should drop low-similarity hits: {body}"
    );
}

// ES `knn.similarity` on l2_norm is an (unsquared) distance: hits with
// l2(query, vector) <= similarity survive, i.e. _score >= 1/(1 + similarity^2).
#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_euclidean_similarity_cutoff_keeps_hits_within_distance(scope: &TestScope) {
    scope
        .create_with_properties(
            json!({ "embedding": { "type": "dense_vector", "dims": 2, "similarity": "l2_norm" } }),
        )
        .await;

    scope
        .index_docs([
            ("exact", json!({ "embedding": [0.0, 0.0] })),
            ("near", json!({ "embedding": [3.0, 0.0] })),
            ("far", json!({ "embedding": [100.0, 0.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [0.0, 0.0],
                "k": 3,
                "similarity": 5.0
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(
        body.hit_ids(),
        vec!["exact", "near"],
        "distance-5 cutoff should keep hits within distance 5: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_euclidean_similarity_cutoff_drops_hits_beyond_distance(scope: &TestScope) {
    scope
        .create_with_properties(
            json!({ "embedding": { "type": "dense_vector", "dims": 2, "similarity": "l2_norm" } }),
        )
        .await;

    scope
        .index_docs([
            ("exact", json!({ "embedding": [0.0, 0.0] })),
            ("far", json!({ "embedding": [1.0, 0.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [0.0, 0.0],
                "k": 2,
                "similarity": 0.5
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(
        body.hit_ids(),
        vec!["exact"],
        "distance-0.5 cutoff should drop the hit at distance 1: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_with_sort_orders_by_field(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "n": { "type": "integer" },
            "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            ("a", json!({ "n": 1, "embedding": [1.0, 0.0] })),
            ("b", json!({ "n": 2, "embedding": [0.9, 0.1] })),
            ("c", json!({ "n": 3, "embedding": [0.0, 1.0] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0],
                "k": 2
            },
            "sort": { "n": "desc" },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids(), vec!["b", "a"], "{body}");
    assert!(body.all_scores_null());
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_with_multi_field_sort(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "x": { "type": "integer" },
            "y": { "type": "integer" },
            "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            ("a", json!({ "x": 1, "y": 1, "embedding": [1.0, 0.0] })),
            ("b", json!({ "x": 1, "y": 2, "embedding": [0.9, 0.1] })),
            ("c", json!({ "x": 0, "y": 9, "embedding": [0.8, 0.2] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0],
                "k": 3
            },
            "sort": [{ "x": "asc" }, { "y": "desc" }],
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids(), vec!["c", "b", "a"], "{body}");
    assert!(body.all_scores_null());
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_with_sort_track_scores_keeps_scores(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "n": { "type": "integer" },
            "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" }
        }))
        .await;

    scope
        .index_docs([
            ("a", json!({ "n": 1, "embedding": [1.0, 0.0] })),
            ("b", json!({ "n": 2, "embedding": [0.9, 0.1] })),
        ])
        .await;

    let body = scope
        .search(json!({
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0],
                "k": 2
            },
            "sort": { "n": "asc" },
            "track_scores": true,
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids(), vec!["a", "b"], "{body}");
    assert!(body.score("a") > 0.0);
    assert!(body.score("a") > body.score("b"));
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_boost_multiplies_hybrid_vector_score(scope: &TestScope) {
    setup_hybrid_docs(scope).await;

    let unboosted = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 2,
                "num_candidates": 3
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let boosted = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 2,
                "num_candidates": 3,
                "boost": 2.0
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert!(boosted.score("vector") > unboosted.score("vector") * 1.5);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_knn_query_combines_lexical_and_vector_scores(scope: &TestScope) {
    setup_hybrid_docs(scope).await;

    let body = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 2,
                "num_candidates": 3
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert_eq!(ids.len(), 3, "{body}");
    assert!(ids.contains(&"both".to_string()));
    assert!(ids.contains(&"vector".to_string()));
    assert!(ids.contains(&"text".to_string()));
    assert!(body.score("both") > body.score("vector") && body.score("both") > body.score("text"));
}

#[test_context(BooksContext)]
#[tokio::test]
async fn ext_knn_maxsim_over_books_ranks_by_token_overlap(books: &BooksContext) {
    let body = books
        .search(json!({
            "knn": { "field": "token_embeddings", "query_vector": [[1.0, 0.0, 0.0, 0.0]], "k": 3 }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(
        body.hit_ids(),
        vec!["lotr", "hobbit", "harry"],
        "MaxSim should rank the fantasy books by their leading token component: {body}"
    );
    assert!(body.score("lotr") > body.score("hobbit"));
    assert!(body.score("hobbit") > body.score("harry"));
}

#[rstest_ctx(TestScope)]
#[case::k_zero(
    json!({ "embedding": { "type": "dense_vector", "dims": 4 } }),
    json!({ "knn": { "field": "embedding", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 0 } })
)]
#[case::num_candidates_less_than_k(
    json!({ "embedding": { "type": "dense_vector", "dims": 4 } }),
    json!({ "knn": { "field": "embedding", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2, "num_candidates": 1 } })
)]
#[case::dev_unindexed_field(
    json!({ "title": { "type": "text" } }),
    json!({ "knn": { "field": "title", "query_vector": [1.0, 0.0], "k": 2 } })
)]
#[case::dev_unknown_field(
    json!({ "embedding": { "type": "dense_vector", "dims": 4 } }),
    json!({ "knn": { "field": "nope", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2 } })
)]
#[case::dev_flat_vector_for_rank_field(
    json!({ "tokens": { "type": "rank_vectors", "dims": 4 } }),
    json!({ "knn": { "field": "tokens", "query_vector": [1.0, 0.0, 0.0, 0.0], "k": 2 } })
)]
#[case::matrix_vector_for_dense_field(
    json!({ "embedding": { "type": "dense_vector", "dims": 4 } }),
    json!({ "knn": { "field": "embedding", "query_vector": [[1.0, 0.0, 0.0, 0.0]], "k": 2 } })
)]
async fn test_knn_search_rejected(
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
async fn test_rank_rrf_with_size_zero_rejected(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 4 }
        }))
        .await;

    let err = scope
        .search(json!({
            "query": { "match_all": {} },
            "knn": {
                "field": "embedding",
                "query_vector": [1.0, 0.0, 0.0, 0.0],
                "k": 2
            },
            "rank": { "rrf": {} },
            "size": 0
        }))
        .await
        .unwrap_err();

    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

// A JSON number that overflows f32 becomes infinity; ES rejects those.
#[rstest_ctx(TestScope)]
#[case::overflow(json!([1e308, 0.0]))]
#[case::just_over_f32_max(json!([1e39, 0.0]))]
async fn test_knn_rejects_non_finite_query_vector(scope: &TestScope, #[case] query_vector: Value) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": 2, "similarity": "cosine" }
        }))
        .await;
    scope
        .index_docs([("1", json!({ "embedding": [1.0, 0.0] }))])
        .await;

    let err = scope
        .search(json!({
            "knn": { "field": "embedding", "query_vector": query_vector, "k": 1 }
        }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}
