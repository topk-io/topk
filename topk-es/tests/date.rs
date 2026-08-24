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

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_fixed_interval(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "over_time": {
                    "date_histogram": { "field": "created", "fixed_interval": "30d", "min_doc_count": 1 }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["over_time"]["buckets"]
        .as_array()
        .unwrap();
    assert_eq!(buckets.len(), 3, "one 30d bucket per doc: {buckets:?}");
    assert_eq!(buckets.iter().map(|b| b["doc_count"].as_u64().unwrap()).sum::<u64>(), 3);
    // Chronological, with an ISO companion on every bucket.
    let keys: Vec<i64> = buckets.iter().map(|b| b["key"].as_i64().unwrap()).collect();
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "not sorted: {keys:?}");
    assert!(buckets
        .iter()
        .all(|b| b["key_as_string"].as_str().unwrap().ends_with('Z')));
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_calendar_month(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "by_month": {
                    "date_histogram": { "field": "created", "calendar_interval": "month", "min_doc_count": 1 }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["by_month"]["buckets"]
        .as_array()
        .unwrap();
    let keys: Vec<&str> = buckets
        .iter()
        .map(|b| b["key_as_string"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        vec![
            "2026-01-01T00:00:00.000Z",
            "2026-06-01T00:00:00.000Z",
            "2026-12-01T00:00:00.000Z"
        ]
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_calendar_year(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "by_year": { "date_histogram": { "field": "created", "calendar_interval": "year" } }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["by_year"]["buckets"]
        .as_array()
        .unwrap();
    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0]["key_as_string"], "2026-01-01T00:00:00.000Z");
    assert_eq!(buckets[0]["doc_count"], 3);
}

#[rstest_ctx(TestScope)]
#[case::both_intervals(json!({ "field": "created", "fixed_interval": "1d", "calendar_interval": "day" }))]
#[case::no_interval(json!({ "field": "created" }))]
#[case::calendar_unit_in_fixed(json!({ "field": "created", "fixed_interval": "1M" }))]
#[case::bad_calendar(json!({ "field": "created", "calendar_interval": "3M" }))]
async fn test_date_histogram_rejected(scope: &TestScope, #[case] body: Value) {
    create_with_dates(scope).await;

    let err = scope
        .search(json!({ "size": 0, "aggs": { "h": { "date_histogram": body } } }))
        .await
        .expect_err("expected 400");
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_math_range_bound(scope: &mut TestScope) {
    create_with_dates(scope).await;

    // Two docs are in the past and one (December 2026) is dated ahead of it.
    let past = scope
        .search_ids(json!({ "range": { "created": { "lte": "now" } } }))
        .await;
    let future = scope
        .search_ids(json!({ "range": { "created": { "gt": "now" } } }))
        .await;
    assert_eq!(past.len() + future.len(), 3);
    assert!(!past.is_empty() && !future.is_empty());

    // ...and nothing is more than a century old.
    let ids = scope
        .search_ids(json!({ "range": { "created": { "lt": "now-100y" } } }))
        .await;
    assert!(ids.is_empty());
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_sort_values_are_epoch_millis(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "created": { "order": "asc" } }]
        }))
        .await
        .expect("search");

    // ES echoes the raw sort value, which for a date field is epoch millis (not ISO).
    let first = &res["hits"]["hits"][0];
    assert_eq!(first["_id"], "1");
    assert_eq!(first["sort"][0].as_i64(), Some(1768471200000), "hit was {first}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_range_with_time_zone(scope: &mut TestScope) {
    create_with_dates(scope).await;

    // Doc 1 is 2026-01-15T10:00Z. A zone-less bound of 2026-01-15T11:00 in +02:00 is 09:00Z,
    // so the doc is after it; the same bound in UTC is 11:00Z, so the doc is before it.
    let after = scope
        .search_ids(json!({
            "range": { "created": { "gte": "2026-01-15T11:00:00", "time_zone": "+02:00" } }
        }))
        .await;
    assert!(after.contains(&"1".to_string()), "got {after:?}");

    let before = scope
        .search_ids(json!({ "range": { "created": { "gte": "2026-01-15T11:00:00" } } }))
        .await;
    assert!(!before.contains(&"1".to_string()), "got {before:?}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_range_accepts_bare_date(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let mut ids = scope
        .search_ids(json!({ "range": { "created": { "gte": "2026-06-01" } } }))
        .await;
    ids.sort();
    assert_eq!(ids, vec!["2", "3"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_range_agg(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "spans": {
                    "date_range": {
                        "field": "created",
                        "ranges": [
                            { "key": "h1", "to": "2026-07-01" },
                            { "key": "h2", "from": "2026-07-01" },
                            // Deliberately overlaps h1/h2: ES counts a doc in every bucket it
                            // matches, so this one covers all three docs.
                            { "key": "all", "from": "2026-01-01" }
                        ]
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["spans"]["buckets"].as_array().unwrap();
    let counts: Vec<(&str, u64)> = buckets
        .iter()
        .map(|b| {
            (
                b["key"].as_str().unwrap(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect();

    assert_eq!(counts, vec![("h1", 2), ("h2", 1), ("all", 3)]);
    // Date bounds echo an ISO companion.
    assert_eq!(buckets[0]["to_as_string"], "2026-07-01T00:00:00.000Z");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_calendar_quarter(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "by_q": { "date_histogram": { "field": "created", "calendar_interval": "quarter", "min_doc_count": 1 } }
            }
        }))
        .await
        .expect("search");

    let keys: Vec<&str> = res["aggregations"]["by_q"]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["key_as_string"].as_str().unwrap())
        .collect();

    // Jan -> Q1, Jun -> Q2, Dec -> Q4.
    assert_eq!(
        keys,
        vec![
            "2026-01-01T00:00:00.000Z",
            "2026-04-01T00:00:00.000Z",
            "2026-10-01T00:00:00.000Z"
        ]
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_time_zone_shifts_buckets(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    // 22:30 UTC on the 14th is 00:30 on the 15th in +02:00, so the day bucket differs by zone.
    scope
        .index_docs(vec![("1", json!({ "created": "2026-01-14T22:30:00.000Z" }))])
        .await;

    let day_bucket = |tz: Option<&str>| {
        let mut body = json!({ "field": "created", "calendar_interval": "day" });
        if let Some(tz) = tz {
            body["time_zone"] = json!(tz);
        }
        json!({ "size": 0, "aggs": { "d": { "date_histogram": body } } })
    };

    let utc = scope.search(day_bucket(None)).await.expect("search");
    assert_eq!(
        utc["aggregations"]["d"]["buckets"][0]["key_as_string"],
        "2026-01-14T00:00:00.000Z"
    );

    let shifted = scope
        .search(day_bucket(Some("+02:00")))
        .await
        .expect("search");
    assert_eq!(
        shifted["aggregations"]["d"]["buckets"][0]["key_as_string"],
        "2026-01-14T22:00:00.000Z",
        "bucket starts at local midnight, reported as the UTC instant"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_named_time_zone_follows_dst(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    // Prague springs forward on 2026-03-29 (+01:00 -> +02:00), so local midnight moves from
    // 23:00Z to 22:00Z across these three local days.
    scope
        .index_docs(vec![
            ("1", json!({ "created": "2026-03-28T22:30:00.000Z" })), // Mar 28 23:30 local
            ("2", json!({ "created": "2026-03-29T10:00:00.000Z" })), // Mar 29 12:00 local
            ("3", json!({ "created": "2026-03-29T22:30:00.000Z" })), // Mar 30 00:30 local
        ])
        .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "d": {
                    "date_histogram": {
                        "field": "created",
                        "calendar_interval": "day",
                        "time_zone": "Europe/Prague"
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["d"]["buckets"].as_array().unwrap();
    let keys: Vec<&str> = buckets
        .iter()
        .map(|b| b["key_as_string"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        vec![
            "2026-03-27T23:00:00.000Z", // local Mar 28
            "2026-03-28T23:00:00.000Z", // local Mar 29, the 23-hour day
            "2026-03-29T22:00:00.000Z", // local Mar 30, midnight now at 22:00Z
        ]
    );
    assert!(buckets.iter().all(|b| b["doc_count"] == 1), "{buckets:?}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_range_accepts_named_time_zone(scope: &mut TestScope) {
    create_with_dates(scope).await;

    // Range bounds resolve named zones too.
    let ids = scope
        .search_ids(json!({
            "range": { "created": { "gte": "2026-01-15T10:30:00", "time_zone": "Europe/Prague" } }
        }))
        .await;
    assert!(ids.contains(&"1".to_string()), "got {ids:?}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_dense_by_default(scope: &mut TestScope) {
    create_with_dates(scope).await;

    // With the default `min_doc_count: 0`, every month between the first and last document is
    // reported, empty ones included: Jan..Dec 2026 is 12 buckets for 3 docs.
    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "m": { "date_histogram": { "field": "created", "calendar_interval": "month" } }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["m"]["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 12, "{buckets:?}");
    assert_eq!(buckets[0]["key_as_string"], "2026-01-01T00:00:00.000Z");
    assert_eq!(buckets[11]["key_as_string"], "2026-12-01T00:00:00.000Z");
    let total: u64 = buckets.iter().map(|b| b["doc_count"].as_u64().unwrap()).sum();
    assert_eq!(total, 3);
    assert_eq!(buckets[1]["doc_count"], 0);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_extended_bounds(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "m": {
                    "date_histogram": {
                        "field": "created",
                        "calendar_interval": "month",
                        "extended_bounds": { "min": "2025-11-01", "max": "2027-02-15" }
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["m"]["buckets"].as_array().unwrap();
    assert_eq!(buckets[0]["key_as_string"], "2025-11-01T00:00:00.000Z");
    assert_eq!(
        buckets.last().unwrap()["key_as_string"],
        "2027-02-01T00:00:00.000Z"
    );
    assert_eq!(buckets.len(), 16, "{buckets:?}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_sub_agg_merge(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({
            "created": { "type": "date" },
            "price": { "type": "integer" }
        }))
        .await;
    // Two docs land in one local Prague day across the UTC midnight between them, so their
    // engine rows merge — exercising the sum/count decomposition of `avg`.
    scope
        .index_docs(vec![
            ("1", json!({ "created": "2026-01-14T23:30:00.000Z", "price": 10 })), // Jan 15 local
            ("2", json!({ "created": "2026-01-15T10:00:00.000Z", "price": 30 })), // Jan 15 local
            ("3", json!({ "created": "2026-01-17T10:00:00.000Z", "price": 5 })),  // Jan 17 local
        ])
        .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "d": {
                    "date_histogram": {
                        "field": "created",
                        "calendar_interval": "day",
                        "time_zone": "Europe/Prague"
                    },
                    "aggs": {
                        "avg_price": { "avg": { "field": "price" } },
                        "sum_price": { "sum": { "field": "price" } }
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["d"]["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 3, "{buckets:?}"); // Jan 15, 16 (empty), 17 local
    assert_eq!(buckets[0]["doc_count"], 2);
    assert_eq!(buckets[0]["avg_price"]["value"], 20.0);
    assert_eq!(buckets[0]["sum_price"]["value"], 40.0);
    // The empty bucket sums to 0 while avg stays null, as in ES.
    assert_eq!(buckets[1]["doc_count"], 0);
    assert_eq!(buckets[1]["sum_price"]["value"], 0.0);
    assert_eq!(buckets[1]["avg_price"]["value"], Value::Null);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_weeks_start_monday(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    // 2026-01-14 is a Wednesday; its week bucket starts on Monday the 12th.
    scope
        .index_docs(vec![("1", json!({ "created": "2026-01-14T10:00:00.000Z" }))])
        .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "w": { "date_histogram": { "field": "created", "calendar_interval": "week" } }
            }
        }))
        .await
        .expect("search");

    assert_eq!(
        res["aggregations"]["w"]["buckets"][0]["key_as_string"],
        "2026-01-12T00:00:00.000Z"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bare_year_range_bound(scope: &mut TestScope) {
    create_with_dates(scope).await;

    // "2026" is the year 2026, not 2026 millis into 1970.
    let ids = scope
        .search_ids(json!({ "range": { "created": { "gte": "2026", "lt": "2026-07" } } }))
        .await;
    assert_eq!(ids.len(), 2, "got {ids:?}");
}
