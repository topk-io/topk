mod common;

use common::TestScope;
use elasticsearch::{BulkOperation, BulkOperations};
use serde_json::json;
use test_context::test_context;

async fn bulk_seed(scope: &TestScope, count: usize) -> u64 {
    let mut ops = BulkOperations::new();
    for i in 0..count {
        ops.push(
            BulkOperation::index(json!({ "title": format!("doc {i}") })).id(format!("seed_{i}")),
        )
        .expect("encode bulk op");
    }

    let body = scope.bulk_without_refresh(ops).await;
    assert_eq!(body["errors"], false, "seed must index every doc: {body}");

    body["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter_map(|item| item["index"]["_seq_no"].as_u64())
        .max()
        .expect("bulk response must report _seq_no")
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_search_waits_for_required_lsn(scope: &TestScope) {
    scope.create().await;

    let lsn = bulk_seed(scope, 100).await;

    let hits = scope
        .search_at_lsn(json!({ "query": { "match_all": {} }, "size": 100 }), lsn)
        .await
        .expect("search at lsn should succeed")
        .hit_ids();

    assert_eq!(
        hits.len(),
        100,
        "every seeded doc must be visible at its lsn"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_search_rejects_unreachable_lsn(scope: &TestScope) {
    scope.create().await;

    let lsn = bulk_seed(scope, 1).await;

    let err = scope
        .search_at_lsn(json!({ "query": { "match_all": {} } }), lsn + 1_000_000_000)
        .await
        .expect_err("an lsn that will never be reached must not be ignored");

    assert!(
        err.status_code().is_server_error() || err.status_code().is_client_error(),
        "expected a failure, got {err:?}"
    );
}

// `_mget` spans indices and LSNs are per-collection, so it cannot honour one. An
// unreachable LSN that `_search` rejects is ignored here — assert that on purpose.
#[test_context(TestScope)]
#[tokio::test]
async fn dev_mget_ignores_required_lsn(scope: &TestScope) {
    scope.create().await;

    let lsn = bulk_seed(scope, 1).await;

    scope
        .search_at_lsn(json!({ "query": { "match_all": {} } }), lsn + 1_000_000_000)
        .await
        .expect_err("_search rejects an unreachable lsn");

    let body = scope
        .request(
            elasticsearch::http::Method::Post,
            &format!("/{}/_mget", scope.name),
            &[("required_lsn", (lsn + 1_000_000_000).to_string())],
            Some(json!({ "ids": ["seed_0"] })),
        )
        .await
        .expect("_mget ignores required_lsn rather than rejecting it");

    assert_eq!(body["docs"][0]["found"], true);
}
