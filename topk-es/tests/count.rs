mod common;

use common::{BooksContext, TestScope};
use test_macros::rstest_ctx;
use elasticsearch::http::StatusCode;
use serde_json::{json, Value};

#[rstest_ctx(TestScope)]
#[case::no_query(None, 0)]
#[case::match_all(Some(json!({ "match_all": {} })), 0)]
async fn test_count_on_empty_index(
    scope: &TestScope,
    #[case] query: Option<Value>,
    #[case] expected: u64,
) {
    scope.create().await;

    assert_eq!(
        scope.count(query).await.expect("count should succeed"),
        expected
    );
}

#[rstest_ctx(BooksContext)]
#[case::no_query(None, 10)]
#[case::fiction(Some(json!({ "term": { "genre": "fiction" } })), 4)]
#[case::fantasy(Some(json!({ "term": { "genre": "fantasy" } })), 3)]
#[case::no_matches(Some(json!({ "term": { "genre": "nope" } })), 0)]
async fn test_count_with_and_without_query(
    books: &BooksContext,
    #[case] query: Option<Value>,
    #[case] expected: u64,
) {
    assert_eq!(
        books.count(query).await.expect("count should succeed"),
        expected
    );
}

#[rstest_ctx(TestScope)]
#[case::unknown_query(json!({ "not_a_real_query": {} }))]
#[case::dev_semantic_query(json!({ "semantic": { "field": "content", "query": "cats" } }))]
async fn test_count_query_rejected(scope: &TestScope, #[case] query: Value) {
    scope.create().await;

    let err = scope.count(Some(query)).await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_ignore_unavailable_missing_index() {
    let client = common::Client::new();
    let missing = format!("ddb-es-proxy-test-{}", uuid::Uuid::new_v4());

    let res = client
        .es()
        .count(elasticsearch::CountParts::Index(&[&missing]))
        .ignore_unavailable(true)
        .body(json!({ "query": { "match_all": {} } }))
        .send()
        .await
        .expect("count");
    assert_eq!(res.status_code(), StatusCode::OK);

    let body: Value = res.json().await.unwrap();
    assert_eq!(body["count"], 0, "{body}");
}
