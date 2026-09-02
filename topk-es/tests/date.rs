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
            (
                "1",
                json!({ "title": "first", "created": "2026-01-15T10:00:00.000Z" }),
            ),
            (
                "2",
                json!({ "title": "second", "created": "2026-06-15T10:00:00.000Z" }),
            ),
            (
                "3",
                json!({ "title": "third", "created": "2026-12-15T10:00:00.000Z" }),
            ),
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
#[case::epoch_millis_bound(json!({ "range": { "created": { "gte": 1782950400000i64 } } }), vec!["3"])]
async fn test_date_range_query(
    scope: &TestScope,
    #[case] query: Value,
    #[case] expected: Vec<&str>,
) {
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

    let buckets = res["aggregations"]["by_date"]["buckets"]
        .as_array()
        .unwrap();
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
    for b in buckets {
        let key = b["key"].as_i64().unwrap();
        assert_eq!(
            key % (30 * 24 * 60 * 60 * 1000),
            0,
            "not epoch-anchored: {b:?}"
        );
    }
    assert_eq!(
        buckets
            .iter()
            .map(|b| b["doc_count"].as_u64().unwrap())
            .sum::<u64>(),
        3
    );
    let keys: Vec<i64> = buckets.iter().map(|b| b["key"].as_i64().unwrap()).collect();
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "not sorted: {keys:?}");
    assert!(buckets
        .iter()
        .all(|b| b["key_as_string"].as_str().unwrap().ends_with('Z')));
}

#[rstest_ctx(TestScope)]
#[case::month("month", vec![("2026-01-01T00:00:00.000Z", 1), ("2026-06-01T00:00:00.000Z", 1), ("2026-12-01T00:00:00.000Z", 1)])]
#[case::quarter("quarter", vec![("2026-01-01T00:00:00.000Z", 1), ("2026-04-01T00:00:00.000Z", 1), ("2026-10-01T00:00:00.000Z", 1)])]
#[case::year("year", vec![("2026-01-01T00:00:00.000Z", 3)])]
async fn test_date_histogram_calendar_interval(
    scope: &TestScope,
    #[case] interval: &str,
    #[case] expected: Vec<(&str, u64)>,
) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "h": {
                    "date_histogram": {
                        "field": "created",
                        "calendar_interval": interval,
                        "min_doc_count": 1
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets: Vec<(&str, u64)> = res["aggregations"]["h"]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["key_as_string"].as_str().unwrap(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(buckets, expected);
}

#[rstest_ctx(TestScope)]
#[case::both_intervals(json!({ "field": "created", "fixed_interval": "1d", "calendar_interval": "day" }))]
#[case::no_interval(json!({ "field": "created" }))]
#[case::calendar_unit_in_fixed(json!({ "field": "created", "fixed_interval": "1M" }))]
#[case::bad_calendar(json!({ "field": "created", "calendar_interval": "3M" }))]
#[case::bad_time_zone(json!({ "field": "created", "calendar_interval": "month", "time_zone": "Mars/Olympus" }))]
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

    let past = scope
        .search_ids(json!({ "range": { "created": { "lte": "now" } } }))
        .await;
    let future = scope
        .search_ids(json!({ "range": { "created": { "gt": "now" } } }))
        .await;
    assert_eq!(past.len() + future.len(), 3);
    assert!(!past.is_empty() && !future.is_empty());

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

    let first = &res["hits"]["hits"][0];
    assert_eq!(first["_id"], "1");
    assert_eq!(
        first["sort"][0].as_i64(),
        Some(1768471200000),
        "hit was {first}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_range_with_time_zone(scope: &mut TestScope) {
    create_with_dates(scope).await;

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
async fn test_date_histogram_time_zone_shifts_buckets(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope
        .index_docs(vec![(
            "1",
            json!({ "created": "2026-01-14T22:30:00.000Z" }),
        )])
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
    let bucket = &shifted["aggregations"]["d"]["buckets"][0];
    assert_eq!(
        bucket["key_as_string"], "2026-01-15T00:00:00.000+02:00",
        "key_as_string renders in the request zone, as ES does"
    );
    assert_eq!(bucket["key"], 1768428000000i64, "key stays the UTC instant");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_range_accepts_named_time_zone(scope: &mut TestScope) {
    create_with_dates(scope).await;

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
    let total: u64 = buckets
        .iter()
        .map(|b| b["doc_count"].as_u64().unwrap())
        .sum();
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
async fn test_date_histogram_sub_aggs(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({
            "created": { "type": "date" },
            "price": { "type": "integer" }
        }))
        .await;
    scope
        .index_docs(vec![
            (
                "1",
                json!({ "created": "2026-01-14T23:30:00.000Z", "price": 10 }),
            ), // Jan 15 local
            (
                "2",
                json!({ "created": "2026-01-15T10:00:00.000Z", "price": 30 }),
            ), // Jan 15 local
            (
                "3",
                json!({ "created": "2026-01-17T10:00:00.000Z", "price": 5 }),
            ), // Jan 17 local
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
                        "time_zone": "+01:00"
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
    scope
        .index_docs(vec![(
            "1",
            json!({ "created": "2026-01-14T10:00:00.000Z" }),
        )])
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

    let ids = scope
        .search_ids(json!({ "range": { "created": { "gte": "2026", "lt": "2026-07" } } }))
        .await;
    assert_eq!(ids.len(), 2, "got {ids:?}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_sub_agg_has_iso_companion(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "y": {
                    "date_histogram": { "field": "created", "calendar_interval": "year" },
                    "aggs": { "newest": { "max": { "field": "created" } } }
                }
            }
        }))
        .await
        .expect("search");

    let bucket = &res["aggregations"]["y"]["buckets"][0];
    assert_eq!(bucket["newest"]["value"].as_f64(), Some(1797328800000.0));
    assert_eq!(
        bucket["newest"]["value_as_string"],
        "2026-12-15T10:00:00.000Z"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_range_bound_rounds_to_end_of_unit(scope: &mut TestScope) {
    create_with_dates(scope).await;

    let lte = scope
        .search_ids(json!({ "range": { "created": { "lte": "2026-01-15" } } }))
        .await;
    assert_eq!(lte, vec!["1"], "lte covers the whole day");

    let lt = scope
        .search_ids(json!({ "range": { "created": { "lt": "2026-01-15" } } }))
        .await;
    assert!(lt.is_empty(), "lt floors to midnight, got {lt:?}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_math_evaluated_in_time_zone(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope
        .index_docs(vec![(
            "1",
            json!({ "created": "2026-06-10T02:00:00.000Z" }),
        )])
        .await;

    let west = scope
        .search_ids(
            json!({ "range": { "created": { "gte": "2026-06-10||/d", "time_zone": "-05:00" } } }),
        )
        .await;
    assert!(west.is_empty(), "got {west:?}");

    let east = scope
        .search_ids(
            json!({ "range": { "created": { "gte": "2026-06-10||/d", "time_zone": "+05:00" } } }),
        )
        .await;
    assert_eq!(east, vec!["1"]);
}

#[rstest_ctx(TestScope)]
#[case::repeated_hour("2026-10-25T02:30:00")]
#[case::skipped_hour("2026-03-29T02:30:00")]
async fn test_bound_in_dst_transition_resolves(scope: &TestScope, #[case] bound: &str) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope
        .index_docs(vec![(
            "1",
            json!({ "created": "2026-12-01T00:00:00.000Z" }),
        )])
        .await;

    let ids = scope
        .search_ids(json!({
            "range": { "created": { "gte": bound, "time_zone": "Europe/Prague" } }
        }))
        .await;
    assert_eq!(ids, vec!["1"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_named_time_zone_buckets_by_local_wall_clock(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope
        .index_docs(vec![
            ("1", json!({ "created": "2026-01-15T10:15:00.000Z" })),
            ("2", json!({ "created": "2026-01-15T10:45:00.000Z" })),
            ("3", json!({ "created": "2026-01-15T12:05:00.000Z" })),
        ])
        .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "h": {
                    "date_histogram": {
                        "field": "created",
                        "calendar_interval": "hour",
                        "time_zone": "Europe/Prague"
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets: Vec<(i64, &str, u64)> = res["aggregations"]["h"]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["key"].as_i64().unwrap(),
                b["key_as_string"].as_str().unwrap(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        buckets,
        vec![
            (1768471200000, "2026-01-15T11:00:00.000+01:00", 2),
            (1768474800000, "2026-01-15T12:00:00.000+01:00", 0),
            (1768478400000, "2026-01-15T13:00:00.000+01:00", 1),
        ]
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_daily_buckets_follow_dst(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope
        .index_docs(vec![
            ("1", json!({ "created": "2026-03-28T12:00:00.000Z" })),
            ("2", json!({ "created": "2026-03-29T12:00:00.000Z" })),
            ("3", json!({ "created": "2026-03-30T12:00:00.000Z" })),
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

    let buckets: Vec<(&str, u64)> = res["aggregations"]["d"]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["key_as_string"].as_str().unwrap(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect();

    assert_eq!(
        buckets,
        vec![
            ("2026-03-28T00:00:00.000+01:00", 1),
            ("2026-03-29T00:00:00.000+01:00", 1),
            ("2026-03-30T00:00:00.000+02:00", 1),
        ]
    );
}

#[rstest_ctx(TestScope)]
async fn test_date_histogram_calendar_interval_fills_empty_buckets(scope: &TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "h": {
                    "date_histogram": { "field": "created", "calendar_interval": "quarter" }
                }
            }
        }))
        .await
        .expect("search");

    let buckets: Vec<(&str, u64)> = res["aggregations"]["h"]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["key_as_string"].as_str().unwrap(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect();

    assert_eq!(
        buckets,
        vec![
            ("2026-01-01T00:00:00.000Z", 1),
            ("2026-04-01T00:00:00.000Z", 1),
            ("2026-07-01T00:00:00.000Z", 0),
            ("2026-10-01T00:00:00.000Z", 1),
        ]
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_fixed_interval_spans_days(scope: &mut TestScope) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "created": { "type": "date" }
        }))
        .await;
    scope
        .index_docs(vec![
            (
                "1",
                json!({ "title": "a", "created": "2026-01-10T10:00:00.000Z" }),
            ),
            (
                "2",
                json!({ "title": "b", "created": "2026-01-20T10:00:00.000Z" }),
            ),
        ])
        .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": { "h": { "date_histogram": {
                "field": "created", "fixed_interval": "30d", "min_doc_count": 1 } } }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["h"]["buckets"].as_array().unwrap();
    assert_eq!(
        buckets.len(),
        1,
        "docs 10d apart share one 30d bucket: {buckets:?}"
    );
    assert_eq!(buckets[0]["doc_count"].as_u64().unwrap(), 2);
    assert_eq!(
        buckets[0]["key_as_string"].as_str().unwrap(),
        "2026-01-07T00:00:00.000Z"
    );
}
