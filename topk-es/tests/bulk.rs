mod common;

use common::{TestScope, TwoIndices};
use topk_test_macros::rstest_ctx;
use elasticsearch::{http::StatusCode, params::Refresh, BulkOperation, BulkOperations, BulkParts};
use serde_json::{json, Value};
use test_context::test_context;

#[test_context(TestScope)]
#[tokio::test]
async fn dev_bulk_mixed_operations(scope: &TestScope) {
    scope.create().await;

    scope
        .index_doc("2", json!({ "title": "before update", "count": 1 }))
        .await;

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::index(json!({ "title": "one" })).id("1"))
        .unwrap();
    ops.push(BulkOperation::update(
        "2",
        json!({ "doc": { "title": "updated" } }),
    ))
    .unwrap();
    ops.push(BulkOperation::<()>::delete("3")).unwrap();
    ops.push(BulkOperation::create(json!({ "title": "nope" })).id("4"))
        .unwrap();
    ops.push(BulkOperation::update(
        "5",
        json!({ "doc": { "title": "nope" }, "doc_as_upsert": true }),
    ))
    .unwrap();

    let body = scope.bulk(ops).await;
    assert_eq!(body["errors"], true, "expected some items to fail: {body}");

    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0]["index"]["status"], 201, "index: {}", items[0]);
    assert_eq!(items[1]["update"]["status"], 200, "update: {}", items[1]);
    assert_eq!(items[2]["delete"]["status"], 200, "delete: {}", items[2]);
    assert_eq!(items[3]["create"]["status"], 400, "create: {}", items[3]);
    assert_eq!(
        items[4]["update"]["status"], 400,
        "doc_as_upsert: {}",
        items[4]
    );

    assert_eq!(
        scope.search_ids(json!({ "match_all": {} })).await,
        vec!["1", "2"],
        "doc 1 indexed, doc 2 updated in place, doc 3 deleted (was never there)"
    );

    let body = scope.get_doc("2").await;
    assert_eq!(body["_source"]["title"], "updated");
    assert_eq!(
        body["_source"]["count"], 1,
        "update should merge, not replace: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bulk_root_endpoint_with_per_item_index(scope: &TestScope) {
    scope.create().await;

    let mut ops = BulkOperations::new();
    ops.push(
        BulkOperation::index(json!({ "title": "via root bulk" }))
            .id("1")
            .index(scope.name.as_str()),
    )
    .unwrap();

    let res = scope
        .client
        .es()
        .bulk(BulkParts::None)
        .refresh(Refresh::WaitFor)
        .body(vec![ops])
        .send()
        .await
        .expect("bulk");
    let body = common::to_json(res).await;
    assert!(body.status.is_success());
    assert_eq!(body["errors"], false, "{body}");

    assert!(scope.get_doc("1").await.status.is_success());
}

#[tokio::test]
async fn test_bulk_root_endpoint_requires_index() {
    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::<Value>::delete("1")).unwrap();

    let client = common::Client::new();
    let res = client
        .es()
        .bulk(BulkParts::None)
        .body(vec![ops])
        .send()
        .await
        .expect("bulk");
    assert_eq!(res.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TwoIndices)]
#[tokio::test]
async fn test_bulk_mixed_index_writes_each(scope: &TwoIndices) {
    let idx_a = &scope.a;
    let idx_b = &scope.b;
    idx_a.create().await;
    idx_b.create().await;

    let mut ops = BulkOperations::new();
    ops.push(
        BulkOperation::index(json!({ "v": 1 }))
            .id("1")
            .index(idx_a.name.as_str()),
    )
    .unwrap();
    ops.push(
        BulkOperation::index(json!({ "v": 2 }))
            .id("2")
            .index(idx_b.name.as_str()),
    )
    .unwrap();
    ops.push(
        BulkOperation::index(json!({ "v": 3 }))
            .id("3")
            .index(idx_a.name.as_str()),
    )
    .unwrap();

    let client = common::Client::new();
    let res = client
        .es()
        .bulk(BulkParts::None)
        .refresh(Refresh::WaitFor)
        .body(vec![ops])
        .send()
        .await
        .expect("bulk");
    let body = common::to_json(res).await;
    assert!(body.status.is_success());
    assert_eq!(body["errors"], false, "{body}");

    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["index"]["_index"], idx_a.name, "{body}");
    assert_eq!(items[0]["index"]["status"], 201, "{body}");
    assert_eq!(items[1]["index"]["_index"], idx_b.name, "{body}");
    assert_eq!(items[1]["index"]["status"], 201, "{body}");
    assert_eq!(items[2]["index"]["_index"], idx_a.name, "{body}");
    assert_eq!(items[2]["index"]["status"], 201, "{body}");

    assert_eq!(
        idx_a.search_ids(json!({ "match_all": {} })).await,
        vec!["1", "3"]
    );
    assert_eq!(
        idx_b.search_ids(json!({ "match_all": {} })).await,
        vec!["2"]
    );
}

#[rstest_ctx(TestScope)]
#[case::dev_scripted(json!({ "script": { "source": "ctx._source.title = 'after'" } }))]
#[case::own_id_in_doc(json!({ "doc": { "_id": "spoofed-id", "title": "after" } }))]
async fn test_bulk_update_rejected_leaves_doc_unchanged(scope: &TestScope, #[case] update: Value) {
    scope.create().await;

    scope.index_doc("1", json!({ "title": "before" })).await;

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::update("1", update)).unwrap();

    let body = scope.bulk(ops).await;
    assert_eq!(body["errors"], true, "{body}");
    assert_eq!(body["items"][0]["update"]["status"], 400, "{body}");

    let body = scope.get_doc("1").await;
    assert_eq!(
        body["_source"]["title"], "before",
        "the rejected update must not have touched the doc: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_bulk_conversion_failure_fails_only_that_item(scope: &TestScope) {
    scope.create().await;

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::index(json!({ "title": "valid" })).id("1"))
        .unwrap();
    ops.push(BulkOperation::index(json!({ "items": [{ "a": 1 }] })).id("2"))
        .unwrap();
    ops.push(BulkOperation::index(json!({ "title": "also valid" })).id("3"))
        .unwrap();

    let body = scope.bulk(ops).await;
    assert_eq!(body["errors"], true, "{body}");

    let items = body["items"].as_array().unwrap();
    assert_eq!(items[0]["index"]["status"], 201, "{body}");
    assert_eq!(items[1]["index"]["status"], 400, "{body}");
    assert_eq!(items[2]["index"]["status"], 201, "{body}");

    assert_eq!(
        scope.search_ids(json!({ "match_all": {} })).await,
        vec!["1", "3"],
        "the rejected item must not block the valid items around it"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bulk_delete_existing_doc_removes_it(scope: &TestScope) {
    scope.create().await;
    scope.index_doc("1", json!({ "title": "to delete" })).await;
    assert_eq!(scope.count(None).await.expect("count should succeed"), 1);

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::<()>::delete("1")).unwrap();

    let body = scope.bulk(ops).await;
    assert_eq!(body["errors"], false, "{body}");
    assert_eq!(body["items"][0]["delete"]["status"], 200, "{body}");

    assert_eq!(scope.count(None).await.expect("count should succeed"), 0);
    assert_eq!(scope.get_doc("1").await.status, StatusCode::NOT_FOUND);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bulk_item_rejection_does_not_block_later_valid_item(scope: &TestScope) {
    scope.create().await;

    let mut ops = BulkOperations::new();
    ops.push(BulkOperation::index(json!({ "_id": "spoofed", "title": "bad" })).id("1"))
        .unwrap();
    ops.push(BulkOperation::index(json!({ "title": "good" })).id("2"))
        .unwrap();

    let body = scope.bulk(ops).await;
    assert_eq!(body["errors"], true, "{body}");
    assert_eq!(body["items"][0]["index"]["status"], 400, "{body}");
    assert_eq!(body["items"][1]["index"]["status"], 201, "{body}");

    assert_eq!(
        scope.search_ids(json!({ "match_all": {} })).await,
        vec!["2"]
    );
}
