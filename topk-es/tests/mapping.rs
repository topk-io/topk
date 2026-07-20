mod common;

use common::TestScope;
use ddb_test_macros::rstest_ctx;
use elasticsearch::{
    http::StatusCode,
    indices::{IndicesCreateParts, IndicesExistsParts, IndicesGetMappingParts, IndicesGetParts},
    BulkOperation, BulkOperations,
};
use serde_json::{json, Value};
use test_context::test_context;

#[rstest_ctx(TestScope)]
#[case::match_text(json!({ "match": { "title": "hello" } }))]
#[case::multi_match(json!({ "multi_match": { "query": "hello", "fields": ["title"] } }))]
#[case::term_keyword(json!({ "term": { "category": "books" } }))]
async fn test_mapped_fields_queryable(scope: &TestScope, #[case] query: Value) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "category": { "type": "keyword" },
            "price": { "type": "integer" },
            "active": { "type": "boolean" },
            "score": { "type": "float" },
            "meta": {
                "type": "object",
                "properties": { "author": { "type": "keyword" } }
            }
        }))
        .await;

    let res = scope
        .index_doc(
            "1",
            json!({
                "title": "hello world of search",
                "category": "books",
                "price": 10,
                "active": true,
                "score": 4.5,
                "meta": { "author": "alice" }
            }),
        )
        .await;
    assert!(res.status.is_success());

    assert_eq!(scope.search_ids(query).await, vec!["1"]);
}

#[rstest_ctx(TestScope)]
#[case::dev_unsupported_type(json!({ "mappings": { "properties": { "created": { "type": "date" } } } }))]
#[case::dev_unknown_field_option(json!({ "mappings": { "properties": { "title": { "type": "text", "analyzer": "standard" } } } }))]
#[case::dev_missing_dims(json!({ "mappings": { "properties": { "v": { "type": "dense_vector" } } } }))]
#[case::dev_bad_mappings_option(json!({ "mappings": { "dynamic": false, "properties": {} } }))]
#[case::dims_too_large(json!({ "mappings": { "properties": { "v": { "type": "dense_vector", "dims": 4294967296i64 } } } }))]
#[case::unindexed_bad_similarity(json!({ "mappings": { "properties": { "v": { "type": "dense_vector", "dims": 4, "index": false, "similarity": "nonsense" } } } }))]
#[case::bit_dims_unaligned(json!({ "mappings": { "properties": { "embedding": { "type": "dense_vector", "dims": 12, "element_type": "bit" } } } }))]
#[case::bit_bad_similarity(json!({ "mappings": { "properties": { "embedding": { "type": "dense_vector", "dims": 32, "element_type": "bit", "similarity": "cosine" } } } }))]
#[case::unknown_element_type(json!({ "mappings": { "properties": { "embedding": { "type": "dense_vector", "dims": 4, "element_type": "nonsense" } } } }))]
#[case::field_mapping_not_object(json!({ "mappings": { "properties": { "title": "text" } } }))]
async fn test_create_rejected(scope: &TestScope, #[case] body: Value) {
    let err = scope.create_with_body(Some(body)).await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);

    let res = scope
        .client
        .es()
        .indices()
        .exists(IndicesExistsParts::Index(&[&scope.name]))
        .send()
        .await
        .expect("exists check");
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
}

#[rstest_ctx(TestScope)]
#[case::keyword(json!({ "category": { "type": "keyword" } }))]
#[case::integer_aliases(json!({
    "a": { "type": "long" },
    "b": { "type": "short" },
    "c": { "type": "byte" }
}))]
#[case::float_aliases(json!({
    "a": { "type": "double" },
    "b": { "type": "half_float" }
}))]
#[case::text_fields(json!({
    "title": {
        "type": "text",
        "fields": { "keyword": { "type": "keyword" } }
    }
}))]
#[case::semantic_text_options(json!({
    "content": {
        "type": "semantic_text",
        "inference_id": ".elser",
        "search_inference_id": ".elser_query",
        "chunking_settings": { "strategy": "sentence" }
    }
}))]
#[case::nested_object(json!({
    "meta": {
        "type": "nested",
        "properties": { "author": { "type": "keyword" } }
    }
}))]
#[case::dense_vector_index_false(
    json!({ "v": { "type": "dense_vector", "dims": 4, "index": false } })
)]
#[case::object_without_properties(json!({ "meta": { "type": "object" } }))]
async fn test_create_accepts_supported_mapping_variants(
    scope: &TestScope,
    #[case] properties: Value,
) {
    scope.create_with_properties(properties).await;
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_text_field_index_false_disables_keyword_index(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "text", "index": false } }))
        .await;

    scope
        .index_doc("1", json!({ "title": "hello world" }))
        .await;

    let err = scope
        .search(json!({ "query": { "match": { "title": "hello" } } }))
        .await;
    assert_eq!(err.unwrap_err().status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn ext_get_mapping_returns_reverse_translated_properties(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "category": { "type": "keyword" },
            "hidden": { "type": "text", "index": false },
            "price": { "type": "long" },
            "meta": {
                "type": "nested",
                "properties": { "author": { "type": "keyword" } }
            },
            "embedding": {
                "type": "dense_vector",
                "dims": 4,
                "similarity": "cosine"
            },
            "bits": {
                "type": "dense_vector",
                "dims": 32,
                "element_type": "bit"
            },
            "tokens": {
                "type": "rank_vectors",
                "dims": 4,
                "quantization": "scalar",
                "top_k": 8,
                "width": 4096
            },
            "content": { "type": "semantic_text" }
        }))
        .await;

    let res = scope
        .client
        .es()
        .indices()
        .get_mapping(IndicesGetMappingParts::Index(&[&scope.name]))
        .send()
        .await
        .expect("get mapping");
    assert!(res.status_code().is_success());
    let body: Value = res.json().await.unwrap();
    let properties = &body[&scope.name]["mappings"]["properties"];

    assert_eq!(
        properties["title"],
        json!({ "type": "text", "index": true }),
        "{body}"
    );
    assert_eq!(
        properties["category"],
        json!({ "type": "keyword", "index": true }),
        "keyword fields must round-trip as keyword, not text: {body}"
    );
    assert_eq!(
        properties["hidden"],
        json!({ "type": "text", "index": false }),
        "{body}"
    );
    assert_eq!(
        properties["price"],
        json!({ "type": "integer", "index": false }),
        "{body}"
    );
    assert_eq!(
        properties["meta"],
        json!({
            "type": "object",
            "properties": { "author": { "type": "keyword", "index": true } }
        }),
        "{body}"
    );
    assert_eq!(
        properties["embedding"],
        json!({
            "type": "dense_vector",
            "dims": 4,
            "similarity": "cosine",
            "index": true,
            "element_type": "float"
        }),
        "{body}"
    );
    assert_eq!(
        properties["bits"],
        json!({
            "type": "dense_vector",
            "dims": 32,
            "similarity": "l2_norm",
            "index": true,
            "element_type": "bit"
        }),
        "{body}"
    );
    assert_eq!(
        properties["tokens"],
        json!({
            "type": "rank_vectors",
            "dims": 4,
            "element_type": "float",
            "index": true,
            "quantization": "scalar",
            "top_k": 8,
            "width": 4096
        }),
        "{body}"
    );
    assert_eq!(
        properties["content"],
        json!({ "type": "semantic_text" }),
        "{body}"
    );
    assert!(properties.get("_id").is_none());
    assert!(properties.get("_embedding.content").is_none());
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_index_returns_mapping_and_settings(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "text" } }))
        .await;

    let res = scope
        .client
        .es()
        .indices()
        .get(IndicesGetParts::Index(&[&scope.name]))
        .send()
        .await
        .expect("get index");
    assert!(res.status_code().is_success());
    let body: Value = res.json().await.unwrap();
    let index = &body[&scope.name];

    assert_eq!(
        index["mappings"]["properties"]["title"],
        json!({ "type": "text", "index": true }),
        "{body}"
    );
    assert_eq!(
        index["settings"]["index"]["provided_name"], scope.name,
        "{body}"
    );
    assert_eq!(
        index["settings"]["index"]["number_of_shards"], "1",
        "{body}"
    );
    assert_eq!(index["aliases"], json!({}), "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_vector_roundtrip(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "embedding": { "type": "dense_vector", "dims": 4, "similarity": "cosine" }
        }))
        .await;

    let res = scope
        .index_doc(
            "1",
            json!({ "title": "hello", "embedding": [0.1, 0.2, 0.3, 0.4] }),
        )
        .await;
    assert!(res.status.is_success());

    let body = scope.get_doc("1").await;
    let embedding = body["_source"]["embedding"].as_array().unwrap();
    assert_eq!(embedding.len(), 4, "{embedding:?}");
    for (got, want) in embedding.iter().zip([0.1, 0.2, 0.3, 0.4]) {
        assert!((got.as_f64().unwrap() - want).abs() < 1e-6);
    }
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_vector_wrong_dimension_rejected(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "embedding": { "type": "dense_vector", "dims": 4 } }))
        .await;

    let res = scope
        .index_doc("1", json!({ "embedding": [0.1, 0.2, 0.3] }))
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[rstest_ctx(TestScope)]
#[case::float(
    json!({ "embedding": { "type": "dense_vector", "dims": 4 } }),
    json!([1.0, 2.0, 3.0, 4.0])
)]
#[case::byte(
    json!({ "embedding": { "type": "dense_vector", "dims": 4, "element_type": "byte" } }),
    json!([0, -128, -56, 127])
)]
async fn dev_vector_via_bulk_roundtrip(
    scope: &TestScope,
    #[case] properties: Value,
    #[case] vector: Value,
) {
    scope.create_with_properties(properties).await;

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::index(json!({ "embedding": vector.clone() })).id("1"))
        .unwrap();

    let body = scope.bulk(ops).await;
    assert_eq!(body["errors"], false, "{body}");

    let body = scope.get_doc("1").await;
    assert_eq!(body["_source"]["embedding"], vector, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_vector_update_merge_preserves_vector(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "embedding": { "type": "dense_vector", "dims": 4 }
        }))
        .await;

    scope
        .index_doc(
            "1",
            json!({ "title": "before", "embedding": [1.0, 2.0, 3.0, 4.0] }),
        )
        .await;

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::update(
        "1",
        json!({ "doc": { "title": "after" } }),
    ))
    .unwrap();

    let body = scope.bulk(ops).await;
    assert!(body.status.is_success());

    let body = scope.get_doc("1").await;
    assert_eq!(body["_source"]["title"], "after");
    assert_eq!(
        body["_source"]["embedding"],
        json!([1.0, 2.0, 3.0, 4.0]),
        "partial update should preserve the vector field: {body}"
    );
}

#[rstest_ctx(TestScope)]
#[case::byte(4, "byte", json!([-1, -128, 1, 0]))]
#[case::bit(32, "bit", json!([-1, 0, 127, -128]))]
async fn dev_dense_vector_element_type_roundtrip(
    scope: &TestScope,
    #[case] dims: u32,
    #[case] element_type: &str,
    #[case] embedding: Value,
) {
    scope
        .create_with_properties(json!({
            "embedding": { "type": "dense_vector", "dims": dims, "element_type": element_type }
        }))
        .await;

    let res = scope
        .index_doc("1", json!({ "embedding": embedding.clone() }))
        .await;
    assert!(res.status.is_success());

    let body = scope.get_doc("1").await;
    assert_eq!(
        body["_source"]["embedding"], embedding,
        "{element_type} vector should round-trip exactly: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_does_not_reinterpret_other_fields_as_signed(scope: &TestScope) {
    scope.create().await;

    scope
        .index_doc("1", json!({ "values": [1, 2, 3, 200] }))
        .await;

    let body = scope.get_doc("1").await;
    assert_eq!(body["_source"]["values"], json!([1, 2, 3, 200]), "{body}");
}

#[rstest_ctx(TestScope)]
#[case::rank_vectors("rank_vectors")]
#[case::ext_matrix("matrix")]
async fn test_rank_vectors_mapping_creation(scope: &TestScope, #[case] type_name: &str) {
    scope
        .create_with_properties(json!({ "tokens": { "type": type_name, "dims": 4 } }))
        .await;
}

#[rstest_ctx(TestScope)]
#[case::float(
    json!({ "type": "rank_vectors", "dims": 4 }),
    json!([[0.1, 0.2, 0.3, 0.4], [0.5, 0.6, 0.7, 0.8]])
)]
#[case::byte(
    json!({ "type": "rank_vectors", "dims": 4, "element_type": "byte" }),
    json!([[0, 1, 2, 255], [128, 64, 32, 16]])
)]
async fn dev_rank_vectors_roundtrip(
    scope: &TestScope,
    #[case] field: Value,
    #[case] matrix: Value,
) {
    scope
        .create_with_properties(json!({ "tokens": field }))
        .await;

    let res = scope
        .index_doc("1", json!({ "tokens": matrix.clone() }))
        .await;
    assert!(res.status.is_success());

    let body = scope.get_doc("1").await;
    let tokens = body["_source"]["tokens"].as_array().unwrap();
    let want_rows = matrix.as_array().unwrap();
    assert_eq!(tokens.len(), want_rows.len(), "{tokens:?}");
    for (row, want_row) in tokens.iter().zip(want_rows) {
        for (got, want) in row
            .as_array()
            .unwrap()
            .iter()
            .zip(want_row.as_array().unwrap())
        {
            assert!((got.as_f64().unwrap() - want.as_f64().unwrap()).abs() < 1e-6);
        }
    }
}

#[tokio::test]
async fn dev_create_index_invalid_name_rejected() {
    let client = common::Client::new();
    for name in ["BadIndex", "1bad", "_bad", "-bad", "bad name"] {
        let res = client
            .es()
            .indices()
            .create(IndicesCreateParts::Index(name))
            .send()
            .await
            .expect("create index");
        assert_eq!(res.status_code(), StatusCode::BAD_REQUEST, "{name:?}");
    }
}
