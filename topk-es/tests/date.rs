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

async fn create_with_docs(scope: &TestScope, docs: Vec<(&str, Value)>) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;
    scope.index_docs(docs).await;
}

fn buckets(res: &Value, name: &str) -> Vec<(String, u64)> {
    res["aggregations"][name]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            (
                b["key_as_string"].as_str().unwrap().to_string(),
                b["doc_count"].as_u64().unwrap(),
            )
        })
        .collect()
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_mapping_round_trip(scope: &TestScope) {
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

#[rstest_ctx(TestScope)]
#[case::iso(json!("2026-01-15T10:00:00.000Z"))]
#[case::epoch_millis(json!(1768471200000i64))]
#[case::naive(json!("2026-01-15T10:00:00"))]
async fn test_date_written_reads_back_as_iso(scope: &TestScope, #[case] created: Value) {
    create_with_docs(scope, vec![("1", json!({ "created": created }))]).await;

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
#[case::bare_date(json!({ "range": { "created": { "gte": "2026-06-01" } } }), vec!["2", "3"])]
#[case::bare_year_and_month(json!({ "range": { "created": { "gte": "2026", "lt": "2026-07" } } }), vec!["1", "2"])]
#[case::lte_covers_whole_day(json!({ "range": { "created": { "lte": "2026-01-15" } } }), vec!["1"])]
#[case::lt_floors_to_midnight(json!({ "range": { "created": { "lt": "2026-01-15" } } }), vec![])]
#[case::term(json!({ "term": { "created": "2026-06-15T10:00:00.000Z" } }), vec!["2"])]
#[case::terms(
    json!({ "terms": { "created": ["2026-01-15T10:00:00.000Z", "2026-12-15T10:00:00.000Z"] } }),
    vec!["1", "3"]
)]
async fn test_date_query(scope: &TestScope, #[case] query: Value, #[case] expected: Vec<&str>) {
    create_with_dates(scope).await;

    let mut ids = scope.search_ids(query).await;
    ids.sort();
    assert_eq!(ids, expected);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_sort(scope: &TestScope) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "created": { "order": "desc" } }]
        }))
        .await
        .expect("search");
    assert_eq!(common::hit_ids(&res), vec!["3", "2", "1"]);
    assert_eq!(
        res["hits"]["hits"][0]["sort"][0].as_i64(),
        Some(1797328800000),
        "sort values are epoch millis"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_unparseable_date_bound_rejected(scope: &TestScope) {
    create_with_dates(scope).await;

    let err = scope
        .search(json!({ "query": { "range": { "created": { "gte": "not-a-date" } } } }))
        .await
        .expect_err("expected 400 for unparseable date bound");
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_terms_agg_has_iso_companion(scope: &TestScope) {
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

#[rstest_ctx(TestScope)]
#[case::month("month", vec![
    ("2026-01-01T00:00:00.000Z", 1),
    ("2026-02-01T00:00:00.000Z", 0),
    ("2026-03-01T00:00:00.000Z", 0),
    ("2026-04-01T00:00:00.000Z", 0),
    ("2026-05-01T00:00:00.000Z", 0),
    ("2026-06-01T00:00:00.000Z", 1),
    ("2026-07-01T00:00:00.000Z", 0),
    ("2026-08-01T00:00:00.000Z", 0),
    ("2026-09-01T00:00:00.000Z", 0),
    ("2026-10-01T00:00:00.000Z", 0),
    ("2026-11-01T00:00:00.000Z", 0),
    ("2026-12-01T00:00:00.000Z", 1),
])]
#[case::quarter("quarter", vec![
    ("2026-01-01T00:00:00.000Z", 1),
    ("2026-04-01T00:00:00.000Z", 1),
    ("2026-07-01T00:00:00.000Z", 0),
    ("2026-10-01T00:00:00.000Z", 1),
])]
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
                "h": { "date_histogram": { "field": "created", "calendar_interval": interval } }
            }
        }))
        .await
        .expect("search");

    let expected: Vec<(String, u64)> = expected
        .into_iter()
        .map(|(k, n)| (k.to_string(), n))
        .collect();
    assert_eq!(buckets(&res, "h"), expected);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_fixed_interval(scope: &TestScope) {
    create_with_docs(
        scope,
        vec![
            ("1", json!({ "created": "2026-01-10T10:00:00.000Z" })),
            ("2", json!({ "created": "2026-01-20T10:00:00.000Z" })),
            ("3", json!({ "created": "2026-03-20T10:00:00.000Z" })),
        ],
    )
    .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": { "h": { "date_histogram": {
                "field": "created", "fixed_interval": "30d", "min_doc_count": 1 } } }
        }))
        .await
        .expect("search");

    assert_eq!(
        buckets(&res, "h"),
        vec![
            ("2026-01-07T00:00:00.000Z".to_string(), 2),
            ("2026-03-08T00:00:00.000Z".to_string(), 1),
        ],
        "epoch-anchored 30d buckets, empty ones pruned"
    );
}

#[rstest_ctx(TestScope)]
#[case::both_intervals(json!({ "field": "created", "fixed_interval": "1d", "calendar_interval": "day" }))]
#[case::no_interval(json!({ "field": "created" }))]
#[case::calendar_unit_in_fixed(json!({ "field": "created", "fixed_interval": "1M" }))]
#[case::bad_calendar(json!({ "field": "created", "calendar_interval": "3M" }))]
#[case::second_is_not_calendar(json!({ "field": "created", "calendar_interval": "second" }))]
#[case::fixed_with_named_time_zone(json!({ "field": "created", "fixed_interval": "1d", "time_zone": "Europe/Prague" }))]
#[case::bare_letter_calendar(json!({ "field": "created", "calendar_interval": "d" }))]
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
async fn test_date_histogram_fixed_interval_with_offset_zone(scope: &TestScope) {
    create_with_docs(
        scope,
        vec![("1", json!({ "created": "2026-01-14T23:00:00.000Z" }))],
    )
    .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": { "d": { "date_histogram": {
                "field": "created", "fixed_interval": "1d", "time_zone": "+02:00" } } }
        }))
        .await
        .expect("search");

    let bucket = &res["aggregations"]["d"]["buckets"][0];
    assert_eq!(bucket["key_as_string"], "2026-01-15T00:00:00.000+02:00");
    assert_eq!(bucket["key"], 1768428000000i64);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_keyed_extended_bounds_without_docs(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "created": { "type": "date" } }))
        .await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": { "m": { "date_histogram": {
                "field": "created",
                "calendar_interval": "month",
                "keyed": true,
                "extended_bounds": { "min": "2026-01-15", "max": 1774915200000i64 }
            } } }
        }))
        .await
        .expect("search");

    let buckets = res["aggregations"]["m"]["buckets"].as_object().unwrap();
    let keys: Vec<&String> = buckets.keys().collect();
    assert_eq!(
        keys,
        [
            "2026-01-01T00:00:00.000Z",
            "2026-02-01T00:00:00.000Z",
            "2026-03-01T00:00:00.000Z"
        ]
    );
    let feb = &buckets["2026-02-01T00:00:00.000Z"];
    assert_eq!(feb["doc_count"], 0);
    assert_eq!(feb["key"], 1769904000000i64);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_math_range_bound(scope: &TestScope) {
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

#[rstest_ctx(TestScope)]
#[case::fixed_offset(Some("+02:00"), "2026-01-15T11:00:00", true)]
#[case::named(Some("Europe/Prague"), "2026-01-15T10:30:00", true)]
#[case::defaults_to_utc(None, "2026-01-15T11:00:00", false)]
async fn test_date_range_time_zone(
    scope: &TestScope,
    #[case] time_zone: Option<&str>,
    #[case] bound: &str,
    #[case] matches_first: bool,
) {
    create_with_dates(scope).await;

    let mut range = json!({ "gte": bound });
    if let Some(tz) = time_zone {
        range["time_zone"] = json!(tz);
    }
    let ids = scope
        .search_ids(json!({ "range": { "created": range } }))
        .await;
    assert_eq!(ids.contains(&"1".to_string()), matches_first, "got {ids:?}");
}

#[rstest_ctx(TestScope)]
#[case::aligned("2025-11-01", "2027-02-01")]
#[case::unaligned("2025-11-15", "2027-02-15")]
async fn test_date_histogram_extended_bounds(
    scope: &TestScope,
    #[case] min: &str,
    #[case] max: &str,
) {
    create_with_dates(scope).await;

    let res = scope
        .search(json!({
            "size": 0,
            "aggs": {
                "m": {
                    "date_histogram": {
                        "field": "created",
                        "calendar_interval": "month",
                        "extended_bounds": { "min": min, "max": max }
                    }
                }
            }
        }))
        .await
        .expect("search");

    let buckets = buckets(&res, "m");
    assert_eq!(buckets.len(), 16, "{buckets:?}");
    assert_eq!(buckets[0].0, "2025-11-01T00:00:00.000Z");
    assert_eq!(buckets[15].0, "2027-02-01T00:00:00.000Z");
    assert_eq!(buckets.iter().map(|(_, n)| n).sum::<u64>(), 3);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_sub_aggs(scope: &TestScope) {
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
    assert_eq!(
        buckets[0]["key"], 1768431600000i64,
        "key stays the UTC instant"
    );
    assert_eq!(
        buckets[0]["key_as_string"], "2026-01-15T00:00:00.000+01:00",
        "key_as_string renders in the request zone, as ES does"
    );
    assert_eq!(buckets[0]["doc_count"], 2);
    assert_eq!(buckets[0]["avg_price"]["value"], 20.0);
    assert_eq!(buckets[0]["sum_price"]["value"], 40.0);
    assert_eq!(buckets[1]["doc_count"], 0);
    assert_eq!(buckets[1]["sum_price"]["value"], 0.0);
    assert_eq!(buckets[1]["avg_price"]["value"], Value::Null);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_date_histogram_weeks_start_monday(scope: &TestScope) {
    create_with_docs(
        scope,
        vec![("1", json!({ "created": "2026-01-14T10:00:00.000Z" }))],
    )
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
async fn test_date_sub_agg_has_iso_companion(scope: &TestScope) {
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
async fn test_date_math_evaluated_in_time_zone(scope: &TestScope) {
    create_with_docs(
        scope,
        vec![("1", json!({ "created": "2026-06-10T02:00:00.000Z" }))],
    )
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
    create_with_docs(
        scope,
        vec![("1", json!({ "created": "2026-12-01T00:00:00.000Z" }))],
    )
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
async fn test_named_time_zone_buckets_by_local_wall_clock(scope: &TestScope) {
    create_with_docs(
        scope,
        vec![
            ("1", json!({ "created": "2026-01-15T10:15:00.000Z" })),
            ("2", json!({ "created": "2026-01-15T10:45:00.000Z" })),
            ("3", json!({ "created": "2026-01-15T12:05:00.000Z" })),
        ],
    )
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
async fn test_daily_buckets_follow_dst(scope: &TestScope) {
    create_with_docs(
        scope,
        vec![
            ("1", json!({ "created": "2026-03-28T12:00:00.000Z" })),
            ("2", json!({ "created": "2026-03-29T12:00:00.000Z" })),
            ("3", json!({ "created": "2026-03-30T12:00:00.000Z" })),
        ],
    )
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

    assert_eq!(
        buckets(&res, "d"),
        vec![
            ("2026-03-28T00:00:00.000+01:00".to_string(), 1),
            ("2026-03-29T00:00:00.000+01:00".to_string(), 1),
            ("2026-03-30T00:00:00.000+02:00".to_string(), 1),
        ]
    );
}
