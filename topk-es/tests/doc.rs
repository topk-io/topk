mod common;

use common::TestScope;
use test_macros::rstest_ctx;
use elasticsearch::{
    http::StatusCode,
    indices::{IndicesCreateParts, IndicesDeleteParts, IndicesExistsParts},
    IndexParts,
};
use serde_json::{json, Value};
use test_context::test_context;

#[test_context(TestScope)]
#[tokio::test]
async fn test_index_get_search_roundtrip(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .index_doc("1", json!({ "title": "hello world", "count": 42 }))
        .await;
    assert!(res.status.is_success());

    let body = scope.get_doc("1").await;
    assert!(body.status.is_success());
    assert_eq!(body["found"], true);
    assert_eq!(body["_id"], "1");
    assert_eq!(body["_source"]["title"], "hello world");
    assert_eq!(body["_source"]["count"], 42);

    assert_eq!(
        scope.search_ids(json!({ "match_all": {} })).await,
        vec!["1"]
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_missing_doc_returns_found_false(scope: &TestScope) {
    scope.create().await;

    let body = scope.get_doc("nonexistent").await;
    assert_eq!(body.status, StatusCode::NOT_FOUND);
    assert_eq!(body["found"], false);
    assert_eq!(body["_id"], "nonexistent");
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_delete_missing_doc_reports_success(scope: &TestScope) {
    scope.create().await;

    let body = scope.delete_doc("nonexistent").await;
    assert!(body.status.is_success());
    assert_eq!(body["result"], "deleted");
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_reindex_returns_created_not_updated(scope: &TestScope) {
    scope.create().await;

    scope.index_doc("1", json!({ "v": 1 })).await;
    let body = scope.index_doc("1", json!({ "v": 2 })).await;
    assert!(body.status.is_success());
    assert_eq!(body["result"], "created");
}

#[tokio::test]
async fn dev_put_to_missing_index() {
    let client = common::Client::new();
    let index = format!("ddb-es-proxy-doc-missing-{}", uuid::Uuid::new_v4());

    let res = client
        .es()
        .index(IndexParts::IndexId(&index, "1"))
        .body(json!({ "v": 1 }))
        .send()
        .await
        .expect("index doc");
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_create_index_twice(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .client
        .es()
        .indices()
        .create(IndicesCreateParts::Index(&scope.name))
        .send()
        .await
        .expect("create index again");
    assert_eq!(res.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_delete_index_then_search_fails(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .client
        .es()
        .indices()
        .delete(IndicesDeleteParts::Index(&[scope.name.as_str()]))
        .send()
        .await
        .expect("delete");
    assert!(res.status_code().is_success());

    let res = scope
        .client
        .es()
        .indices()
        .delete(IndicesDeleteParts::Index(&[scope.name.as_str()]))
        .send()
        .await
        .expect("delete again");
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_boolean_roundtrip(scope: &TestScope) {
    scope.create().await;

    scope
        .index_doc("1", json!({ "active": true, "disabled": false }))
        .await;

    let body = scope.get_doc("1").await;
    assert_eq!(body["_source"]["active"], true);
    assert_eq!(body["_source"]["disabled"], false);
}

#[rstest_ctx(TestScope)]
#[case::dev_array_of_objects("1".to_string(), json!({ "items": [{"a": 1}, {"a": 2}] }))]
#[case::dev_array_of_booleans("1".to_string(), json!({ "flags": [true, false] }))]
#[case::mixed_type_array("1".to_string(), json!({ "mixed": [1, "two", 3] }))]
#[case::id_too_long("a".repeat(600), json!({ "v": 1 }))]
async fn test_index_doc_rejected(scope: &TestScope, #[case] id: String, #[case] body: Value) {
    scope.create().await;

    let res = scope.index_doc(&id, body).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn ext_array_of_arrays_roundtrips_as_matrix(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .index_doc("1", json!({ "items": [[1, 2], [3, 4]] }))
        .await;
    assert!(res.status.is_success());

    let body = scope.get_doc("1").await;
    assert_eq!(body["_source"]["items"], json!([[1.0, 2.0], [3.0, 4.0]]));
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_empty_array_roundtrip(scope: &TestScope) {
    scope.create().await;

    let res = scope.index_doc("1", json!({ "tags": [] })).await;
    assert!(res.status.is_success());

    let body = scope.get_doc("1").await;
    assert_eq!(body["_source"]["tags"], json!([]), "{body}");
}

#[rstest_ctx(TestScope)]
#[case::false_omits_key(
    json!({ "title": "hello", "n": 1 }),
    Some(&["false"][..]), None, None,
    None
)]
#[case::includes_filters_fields(
    json!({ "title": "hello", "n": 1 }),
    None, Some(&["title"][..]), None,
    Some(json!({ "title": "hello" }))
)]
#[case::excludes_drops_fields(
    json!({ "title": "hello", "n": 1 }),
    None, None, Some(&["n"][..]),
    Some(json!({ "title": "hello" }))
)]
#[case::bare_csv_is_includes_shorthand(
    json!({ "title": "hello", "n": 1 }),
    Some(&["title"][..]), None, None,
    Some(json!({ "title": "hello" }))
)]
#[case::includes_then_excludes(
    json!({ "a": 1, "b": 2, "c": 3 }),
    None, Some(&["a", "b"][..]), Some(&["b"][..]),
    Some(json!({ "a": 1 }))
)]
async fn test_get_doc_source_filtering(
    scope: &TestScope,
    #[case] doc: Value,
    #[case] source: Option<&[&str]>,
    #[case] includes: Option<&[&str]>,
    #[case] excludes: Option<&[&str]>,
    #[case] expected: Option<Value>,
) {
    scope.create().await;
    scope.index_doc("1", doc).await;

    let body = scope
        .get_doc_with_source("1", source, includes, excludes)
        .await;
    assert!(body.status.is_success());
    match expected {
        Some(expected) => assert_eq!(body["_source"], expected, "{body}"),
        None => assert!(!body.as_object().unwrap().contains_key("_source")),
    }
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_source_returns_bare_source(scope: &TestScope) {
    scope.create().await;
    scope
        .index_doc("1", json!({ "title": "hello", "n": 1 }))
        .await;

    let body = scope.get_source_with_source("1", None, None, None).await;
    assert!(body.status.is_success());
    assert_eq!(*body, json!({ "title": "hello", "n": 1 }), "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_source_missing_doc_returns_error_envelope(scope: &TestScope) {
    scope.create().await;

    let body = scope
        .get_source_with_source("nonexistent", None, None, None)
        .await;
    assert_eq!(body.status, StatusCode::NOT_FOUND);
    assert_eq!(body["status"], 404, "{body}");
    assert_eq!(
        body["error"]["type"], "resource_not_found_exception",
        "{body}"
    );
    assert_eq!(
        body["error"]["root_cause"][0]["type"], "resource_not_found_exception",
        "{body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_source_filters_fields(scope: &TestScope) {
    scope.create().await;
    scope
        .index_doc("1", json!({ "title": "hello", "n": 1, "tag": "x" }))
        .await;

    let body = scope
        .get_source_with_source("1", None, Some(&["title", "n"]), Some(&["n"]))
        .await;
    assert!(body.status.is_success());
    assert_eq!(*body, json!({ "title": "hello" }), "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_get_source_false_is_rejected(scope: &TestScope) {
    scope.create().await;
    scope.index_doc("1", json!({ "title": "hello" })).await;

    let body = scope
        .get_source_with_source("1", Some(&["false"]), None, None)
        .await;
    assert_eq!(body.status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_doc_body_with_own_id_rejected(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .index_doc("real-id", json!({ "_id": "spoofed-id", "name": "test" }))
        .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);

    assert_eq!(scope.get_doc("real-id").await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        scope.get_doc("spoofed-id").await.status,
        StatusCode::NOT_FOUND
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_head_index_exists(scope: &TestScope) {
    scope.create().await;

    let res = scope
        .client
        .es()
        .indices()
        .exists(IndicesExistsParts::Index(&[&scope.name]))
        .send()
        .await
        .expect("head");
    assert_eq!(res.status_code(), StatusCode::OK);

    scope
        .client
        .es()
        .indices()
        .delete(IndicesDeleteParts::Index(&[scope.name.as_str()]))
        .send()
        .await
        .expect("delete");

    let res = scope
        .client
        .es()
        .indices()
        .exists(IndicesExistsParts::Index(&[&scope.name]))
        .send()
        .await
        .expect("head");
    assert_eq!(res.status_code(), StatusCode::NOT_FOUND);
}
