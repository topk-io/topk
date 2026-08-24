mod common;

use common::TestScope;
use elasticsearch::{http::StatusCode, indices::IndicesGetMappingParts};
use serde_json::{json, Value};
use test_context::test_context;
use test_macros::rstest_ctx;

async fn create_with_dates(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "created": { "type": "date" }
        }))
        .await;

    scope
        .index_docs(vec![
            ("1", json!({ "title": "first", "created": "2026-01-15T10:00:00.000Z" })),
            ("2", json!({ "title": "second", "created": "2026-06-15T10:00:00.000Z" })),
            ("3", json!({ "title": "third", "created": "2026-12-15T10:00:00.000Z" })),
        ])
        .await;
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_mapping_round_trip(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;

    let res = scope
        .client
        .es()
        .indices()
        .get_mapping(IndicesGetMappingParts::Index(&[&scope.name]))
        .send()
        .await
        .expect("get mapping");
    let body: Value = res.json().await.unwrap();
    let properties = &body[&scope.name]["mappings"]["properties"];

    assert_eq!(properties["created"]["type"], "date");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_written_as_iso_reads_back_as_iso(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope.get_doc("1").await;
    assert_eq!(res["_source"]["created"], "2026-01-15T10:00:00.000Z");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_written_as_epoch_millis(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope
        .index_docs(vec![("1", json!({ "created": 1768471200000i64 }))])
        .await;

    // Epoch millis are stored verbatim and still read back as ISO-8601.
    let res = scope.get_doc("1").await;
    assert_eq!(res["_source"]["created"], "2026-01-15T10:00:00.000Z");
}

#[rstest_ctx(TestScope)]
#[case::gte(json!({ "range": { "created": { "gte": "2026-06-01T00:00:00.000Z" } } }), vec!["2", "3"])]
#[case::lt(json!({ "range": { "created": { "lt": "2026-06-01T00:00:00.000Z" } } }), vec!["1"])]
#[case::between(
    json!({ "range": { "created": { "gte": "2026-02-01T00:00:00.000Z", "lte": "2026-07-01T00:00:00.000Z" } } }),
    vec!["2"]
)]
// 2026-07-02T00:00:00Z as raw epoch millis: only the December doc is after it.
#[case::epoch_millis_bound(json!({ "range": { "created": { "gte": 1782950400000i64 } } }), vec!["3"])]
async fn test_date_range_query(scope: &TestScope, #[case] query: Value, #[case] expected: Vec<&str>) {
    create_with_dates(scope).await;

    let mut ids = scope.search_ids(query).await;
    ids.sort();
    assert_eq!(ids, expected);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_term_query(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let ids = scope
        .search_ids(json!({ "term": { "created": "2026-06-15T10:00:00.000Z" } }))
        .await;
    assert_eq!(ids, vec!["2"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_sort(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "created": { "order": "desc" } }]
        }))
        .await
        .expect("search");
    assert_eq!(common::hit_ids(&res), vec!["3", "2", "1"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_unparseable_date_bound_rejected(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let err = scope
        .search(json!({ "query": { "range": { "created": { "gte": "not-a-date" } } } }))
        .await
        .expect_err("expected 400 for unparseable date bound");
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_terms_query(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let mut ids = scope
        .search_ids(json!({
            "terms": { "created": ["2026-01-15T10:00:00.000Z", "2026-12-15T10:00:00.000Z"] }
        }))
        .await;
    ids.sort();
    assert_eq!(ids, vec!["1", "3"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_metric_agg_has_iso_companion(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": { "newest": { "max": { "field": "created" } } }
        }))
        .await
        .expect("search");

    // metric agg values are JSON numbers (f64), as in ES
    assert_eq!(
        res["aggregations"]["newest"]["value"].as_f64(),
        Some(1797328800000.0)
    );
    assert_eq!(
        res["aggregations"]["newest"]["value_as_string"],
        "2026-12-15T10:00:00.000Z"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_terms_agg_has_iso_companion(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": { "by_date": { "terms": { "field": "created" } } }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["by_date"]["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 3);
    for bucket in buckets {
        assert!(bucket["key"].is_number(), "key should be epoch millis");
        assert!(
            bucket["key_as_string"].as_str().unwrap().ends_with("Z"),
            "expected ISO companion, got {bucket}"
        );
    }
}
