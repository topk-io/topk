mod common;

use common::{BooksContext, TestScope};
use test_macros::rstest_ctx;
use elasticsearch::http::StatusCode;
use serde_json::{json, Value};
use test_context::test_context;

async fn setup_bool_scoring_docs(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "body": { "type": "text" },
            "title": { "type": "text" },
            "genre": { "type": "keyword" },
            "n": { "type": "integer" }
        }))
        .await;

    scope
        .index_docs([
            (
                "1",
                json!({ "body": "cats", "title": "alpha", "genre": "fantasy", "n": 2 }),
            ),
            (
                "2",
                json!({ "body": "cats", "title": "beta", "genre": "fantasy", "n": 1 }),
            ),
            (
                "3",
                json!({ "body": "dogs", "title": "alpha", "genre": "sci-fi", "n": 3 }),
            ),
            (
                "4",
                json!({ "body": "cats", "title": "gamma", "genre": "fantasy", "n": 4 }),
            ),
        ])
        .await;
}

#[rstest_ctx(BooksContext)]
#[case::term(json!({ "term": { "genre": "fantasy" } }), vec!["harry", "hobbit", "lotr"])]
#[case::terms(
    json!({ "terms": { "genre": ["fantasy", "romance"] } }),
    vec!["harry", "hobbit", "lotr", "pride"]
)]
#[case::range_lower_bound(
    json!({ "range": { "published_year": { "gte": 1950 } } }),
    vec!["alchemist", "catcher", "harry", "lotr", "mockingbird"]
)]
#[case::range_open_closed(
    json!({ "range": { "published_year": { "gt": 1949, "lte": 1960 } } }),
    vec!["catcher", "lotr", "mockingbird"]
)]
#[case::exists(
    json!({ "exists": { "field": "embedding" } }),
    vec![
        "alchemist",
        "catcher",
        "gatsby",
        "harry",
        "hobbit",
        "lotr",
        "moby",
        "mockingbird",
        "nineteen_eighty_four",
        "pride"
    ]
)]
#[case::dev_prefix(json!({ "prefix": { "title": "Pride" } }), vec!["pride"])]
#[case::dev_prefix_value(
    json!({ "prefix": { "title": { "value": "Pride" } } }),
    vec!["pride"]
)]
#[case::dev_regexp(
    json!({ "regexp": { "title": "Moby Dick|1984" } }),
    vec!["moby", "nineteen_eighty_four"]
)]
#[case::ids(
    json!({ "ids": { "values": ["hobbit", "lotr", "nonexistent"] } }),
    vec!["hobbit", "lotr"]
)]
#[case::bool_must_filter(
    json!({
        "bool": {
            "must": [{ "term": { "genre": "fantasy" } }],
            "filter": [{ "range": { "published_year": { "gte": 1950 } } }]
        }
    }),
    vec!["harry", "lotr"]
)]
#[case::bool_must_not(
    json!({
        "bool": { "must_not": [{ "term": { "genre": "fantasy" } }] }
    }),
    vec![
        "alchemist",
        "catcher",
        "gatsby",
        "moby",
        "mockingbird",
        "nineteen_eighty_four",
        "pride"
    ]
)]
#[case::bool_should_without_required(
    json!({
        "bool": {
            "should": [
                { "term": { "genre": "romance" } },
                { "term": { "genre": "adventure" } }
            ]
        }
    }),
    vec!["moby", "pride"]
)]
async fn test_query_dsl(books: &BooksContext, #[case] query: Value, #[case] expected: Vec<&str>) {
    assert_eq!(books.search_ids(query).await, expected);
}

#[rstest_ctx(TestScope)]
#[case::prefix_case_insensitive(
    json!({ "query": { "prefix": { "title": { "value": "hob", "case_insensitive": true } } } })
)]
#[case::regexp_flags(
    json!({ "query": { "regexp": { "title": { "value": "hob.*", "flags": "ALL" } } } })
)]
#[case::bool_minimum_should_match(json!({
    "query": {
        "bool": {
            "should": [{ "term": { "genre": "fantasy" } }],
            "minimum_should_match": 1
        }
    }
}))]
#[case::match_minimum_should_match(json!({
    "query": {
        "match": {
            "title": {
                "query": "hello",
                "minimum_should_match": 1
            }
        }
    }
}))]
async fn dev_query_dsl_rejected(scope: &TestScope, #[case] body: Value) {
    scope.create().await;

    let err = scope.search(body).await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

// ES coerces numeric ids to their string form, so `ids` accepts them too.
#[test_context(TestScope)]
#[tokio::test]
async fn test_ids_query_coerces_numeric_values(scope: &TestScope) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "title": "one" })),
            ("2", json!({ "title": "two" })),
            ("3", json!({ "title": "three" })),
        ])
        .await;

    let ids = scope
        .search_ids(json!({ "ids": { "values": [1, 2] } }))
        .await;
    assert_eq!(ids, vec!["1", "2"]);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_regexp_case_insensitive_matches(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "keyword" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "title": "Hobbit" })),
            ("2", json!({ "title": "hobnob" })),
            ("3", json!({ "title": "Other" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": {
                "regexp": {
                    "title": {
                        "value": "hob.*",
                        "case_insensitive": true
                    }
                }
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert_eq!(ids.len(), 2, "{body}");
    assert!(ids.contains(&"1".to_string()));
    assert!(ids.contains(&"2".to_string()));
}

#[rstest_ctx(TestScope)]
#[case::dev_term_keyword(json!({ "term": { "category.keyword": "books" } }), vec!["1"])]
#[case::term_object_with_boost(
    json!({ "term": { "category": { "value": "books", "boost": 2.0 } } }),
    vec!["1"]
)]
#[case::dev_terms_keyword_with_boost(
    json!({ "terms": { "category.keyword": ["books"], "boost": 2.0 } }),
    vec!["1"]
)]
#[case::range_open_closed(json!({ "range": { "price": { "gt": 5, "lte": 10 } } }), vec!["1"])]
#[case::match_operator_and(
    json!({ "match": { "title": { "query": "hello world", "operator": "and" } } }),
    vec!["1"]
)]
#[case::bool_single_object_clause(
    json!({ "bool": { "must": { "term": { "category": "books" } } } }),
    vec!["1"]
)]
#[case::bool_empty_matches_all(json!({ "bool": {} }), vec!["1", "2"])]
#[case::bool_required_clause_makes_should_optional(
    json!({
        "bool": {
            "must": { "term": { "category": "books" } },
            "should": [{ "term": { "category": "electronics" } }]
        }
    }),
    vec!["1"]
)]
async fn test_query_dsl_variants(
    scope: &TestScope,
    #[case] query: Value,
    #[case] expected: Vec<&str>,
) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "category": { "type": "keyword" },
            "price": { "type": "integer" }
        }))
        .await;
    scope
        .index_docs([
            (
                "1",
                json!({ "title": "hello world", "category": "books", "price": 10 }),
            ),
            (
                "2",
                json!({ "title": "hello", "category": "electronics", "price": 20 }),
            ),
        ])
        .await;
    assert_eq!(scope.count(None).await.expect("count should succeed"), 2);

    let expected = expected
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    assert_eq!(scope.search_ids(query).await, expected);
}

#[rstest_ctx(TestScope)]
#[case::dev_multi_match_empty_fields(
    json!({ "query": { "multi_match": { "query": "hello", "fields": [] } } })
)]
#[case::invalid_sort_order(json!({ "query": { "match_all": {} }, "sort": [{ "price": "sideways" }] }))]
#[case::range_object_bound(json!({ "query": { "range": { "price": { "gte": { "a": 1 } } } } }))]
async fn test_search_request_rejected(scope: &TestScope, #[case] body: Value) {
    scope.create().await;

    let err = scope.search(body).await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

// ES parses a range bound at the shard level, so this needs both a real mapping
// and a document — an unmapped field short-circuits to match_none, and an empty
// index has no shard to parse on.
#[test_context(TestScope)]
#[tokio::test]
async fn test_range_array_bound_rejected(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "price": { "type": "integer" } }))
        .await;
    scope.index_doc("1", json!({ "price": 10 })).await;

    let err = scope
        .search(json!({ "query": { "range": { "price": { "lt": [1, 2] } } } }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

// We reject a non-scalar bound while parsing the request, so it fails whatever
// the index looks like. ES only parses bounds at the shard level, so on an index
// with no mapping and no documents it short-circuits to match_none and returns
// 200 instead.
#[test_context(TestScope)]
#[tokio::test]
async fn dev_range_array_bound_rejected_on_bare_index(scope: &TestScope) {
    scope.create().await;

    let err = scope
        .search(json!({ "query": { "range": { "price": { "lt": [1, 2] } } } }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[test_context(TestScope)]
#[tokio::test]
async fn bug_size_limited_total_reports_returned_hits(scope: &TestScope) {
    scope.create().await;

    let docs: Vec<(String, Value)> = (0..5).map(|i| (i.to_string(), json!({ "n": i }))).collect();
    scope
        .index_docs(docs.iter().map(|(id, b)| (id.as_str(), b.clone())))
        .await;
    assert_eq!(scope.count(None).await.expect("count should succeed"), 5);

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "size": 2 }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids().len(), 2, "{body}");
    assert_eq!(body.total(), 2, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn bug_size_zero_returns_no_hits(scope: &TestScope) {
    scope.create().await;

    scope.index_docs([("1", json!({ "n": 1 }))]).await;

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "size": 0 }))
        .await
        .expect("size: 0 should succeed, not be rejected");
    assert_eq!(body.hit_ids().len(), 0, "{body}");
    assert_eq!(body.total(), 0, "{body}");
}

// ES serves `_search` over GET as well as POST, body in either.
#[test_context(TestScope)]
#[tokio::test]
async fn test_search_over_get(scope: &TestScope) {
    scope.create().await;
    scope.index_docs([("1", json!({ "n": 1 }))]).await;

    let res = scope
        .client
        .es()
        .send(
            elasticsearch::http::Method::Get,
            &format!("/{}/_search", scope.name),
            elasticsearch::http::headers::HeaderMap::new(),
            None::<&Value>,
            Some(elasticsearch::http::request::JsonBody::new(
                json!({ "query": { "match_all": {} } }),
            )),
            None,
        )
        .await
        .expect("GET _search");
    assert_eq!(res.status_code(), StatusCode::OK);

    let body: Value = res.json().await.unwrap();
    assert_eq!(body["hits"]["hits"].as_array().unwrap().len(), 1, "{body}");
}

// `ignore_unavailable=true` turns a missing index into an empty result.
#[tokio::test]
async fn test_ignore_unavailable_missing_index() {
    let client = common::Client::new();
    let missing = format!("ddb-es-proxy-test-{}", uuid::Uuid::new_v4());

    let res = client
        .es()
        .search(elasticsearch::SearchParts::Index(&[&missing]))
        .ignore_unavailable(true)
        .body(json!({ "query": { "match_all": {} } }))
        .send()
        .await
        .expect("search");
    assert_eq!(res.status_code(), StatusCode::OK);

    let body: Value = res.json().await.unwrap();
    assert_eq!(body["hits"]["hits"].as_array().unwrap().len(), 0, "{body}");
}

// ES disables fielddata on `_id`, so it cannot sort on it. TopK stores `_id` as
// an ordinary column, so we can.
#[test_context(TestScope)]
#[tokio::test]
async fn ext_sort_on_id(scope: &TestScope) {
    scope.create().await;
    scope
        .index_docs([
            ("b", json!({ "n": 1 })),
            ("a", json!({ "n": 2 })),
            ("c", json!({ "n": 3 })),
        ])
        .await;

    let body = scope
        .search(json!({ "sort": ["_id"] }))
        .await
        .expect("sort on _id should succeed");
    assert_eq!(body.hit_ids(), vec!["a", "b", "c"], "{body}");
}

// A range with no bounds is ES's field-exists check.
#[test_context(TestScope)]
#[tokio::test]
async fn test_range_no_bounds_matches_field_exists(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "n": { "type": "integer" }, "g": { "type": "keyword" } }))
        .await;
    scope
        .index_docs([
            ("1", json!({ "n": 5, "g": "a" })),
            ("2", json!({ "g": "b" })),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "range": { "n": {} } } }))
        .await
        .expect("bound-less range should succeed");
    assert_eq!(body.hit_ids(), vec!["1"], "{body}");
}

// A TopK column holds several value types natively, so matching is type-exact
// rather than coercing. ES casts the query value to the field type and matches.
#[rstest_ctx(TestScope)]
#[case::dev_bool_from_string(json!({ "b": { "type": "boolean" } }), json!({ "b": true }), json!({ "b": "true" }))]
#[case::dev_int_from_string(json!({ "n": { "type": "integer" } }), json!({ "n": 5 }), json!({ "n": "5" }))]
async fn test_term_is_type_exact(
    scope: &TestScope,
    #[case] properties: Value,
    #[case] doc: Value,
    #[case] query: Value,
) {
    scope.create_with_properties(properties).await;
    scope.index_docs([("1", doc)]).await;

    let body = scope
        .search(json!({ "query": { "term": query } }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids().len(), 0, "{body}");
}

// `_score` sorts descending without an explicit order, unlike every other field,
// and in both the bare and object forms.
#[rstest_ctx(TestScope)]
#[case::bare(json!(["_score"]), vec!["2", "1"])]
#[case::object_without_order(json!([{ "_score": {} }]), vec!["2", "1"])]
#[case::explicit_asc(json!([{ "_score": { "order": "asc" } }]), vec!["1", "2"])]
#[case::explicit_desc(json!([{ "_score": { "order": "desc" } }]), vec!["2", "1"])]
async fn test_sort_on_score(scope: &TestScope, #[case] sort: Value, #[case] expected: Vec<&str>) {
    scope
        .create_with_properties(json!({ "t": { "type": "text" } }))
        .await;
    scope
        .index_docs([
            ("1", json!({ "t": "cats" })),
            ("2", json!({ "t": "cats cats cats" })),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "match": { "t": "cats" } }, "sort": sort }))
        .await
        .expect("sort on _score should succeed");
    assert_eq!(body.hit_ids(), expected, "{body}");
}

// A lone descending `_score` is the default ordering, so ES attaches no sort
// values to the hits. Any other sort, including `_score` ascending, does.
#[rstest_ctx(TestScope)]
#[case::bare(json!(["_score"]), true)]
#[case::object_without_order(json!([{ "_score": {} }]), true)]
#[case::explicit_desc(json!([{ "_score": { "order": "desc" } }]), true)]
#[case::explicit_asc(json!([{ "_score": { "order": "asc" } }]), false)]
#[case::score_then_field(json!(["_score", "n"]), false)]
#[case::field(json!(["n"]), false)]
async fn test_sort_values_omitted_for_default_order(
    scope: &TestScope,
    #[case] sort: Value,
    #[case] omitted: bool,
) {
    scope
        .create_with_properties(json!({ "t": { "type": "text" }, "n": { "type": "integer" } }))
        .await;
    scope.index_doc("1", json!({ "t": "cats", "n": 5 })).await;

    let body = scope
        .search(json!({ "query": { "match": { "t": "cats" } }, "sort": sort }))
        .await
        .expect("search should succeed");
    assert_eq!(body.all_sort_omitted(), omitted, "{body}");
}

#[rstest_ctx(TestScope)]
#[case::bare_string(json!("n"), vec!["2", "3", "1"])]
#[case::desc(json!([{ "n": "desc" }]), vec!["1", "3", "2"])]
#[case::nested_order(json!([{ "n": { "order": "asc" } }]), vec!["2", "3", "1"])]
async fn test_sort_single_field(
    scope: &TestScope,
    #[case] sort: Value,
    #[case] expected: Vec<&str>,
) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "n": 3 })),
            ("2", json!({ "n": 1 })),
            ("3", json!({ "n": 2 })),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "sort": sort }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), expected, "{body}");
}

#[rstest_ctx(TestScope)]
#[case::first_page(0, 2, vec!["2", "3"])]
#[case::second_page(2, 2, vec!["1", "4"])]
#[case::partial_last_page(3, 2, vec!["4"])]
#[case::past_end(5, 2, vec![])]
async fn test_from_size_pagination(
    scope: &TestScope,
    #[case] from: u64,
    #[case] size: u64,
    #[case] expected: Vec<&str>,
) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "n": 3 })),
            ("2", json!({ "n": 1 })),
            ("3", json!({ "n": 2 })),
            ("4", json!({ "n": 4 })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "n": "asc" }],
            "from": from,
            "size": size,
        }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), expected, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_from_plus_size_over_max_result_window_rejected(scope: &TestScope) {
    scope.create().await;

    let err = scope
        .search(json!({ "query": { "match_all": {} }, "from": 9_999, "size": 2 }))
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
}

#[rstest_ctx(TestScope)]
#[case::unsupported_option(json!([{ "n": { "order": "asc", "mode": "min" } }]))]
#[case::missing_first(json!([{ "n": { "order": "asc", "missing": "_first" } }]))]
#[case::too_many_fields(json!([
    { "f1": "asc" }, { "f2": "asc" }, { "f3": "asc" }, { "f4": "asc" },
    { "f5": "asc" }, { "f6": "asc" }, { "f7": "asc" }, { "f8": "asc" },
    { "f9": "asc" }
]))]
async fn dev_sort_rejected(scope: &TestScope, #[case] sort: Value) {
    scope.create().await;

    let err = scope
        .search(json!({ "query": { "match_all": {} }, "sort": sort }))
        .await;
    assert_eq!(err.unwrap_err().status_code(), StatusCode::BAD_REQUEST);
}

// Docs missing a sort field already sort last, so `missing: _last` is a no-op;
// `_first` would change the order and stays rejected.
#[test_context(TestScope)]
#[tokio::test]
async fn test_sort_missing_last_accepted(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "n": { "type": "integer" } }))
        .await;
    scope
        .index_docs([
            ("1", json!({ "n": 2 })),
            ("2", json!({})),
            ("3", json!({ "n": 1 })),
        ])
        .await;

    let body = scope
        .search(json!({ "sort": [{ "n": { "order": "asc", "missing": "_last" } }] }))
        .await
        .expect("missing: _last should be accepted");
    assert_eq!(body.hit_ids(), vec!["3", "1", "2"], "{body}");
}

#[rstest_ctx(TestScope)]
#[case::asc_then_desc(json!([{ "a": "asc" }, { "b": "desc" }]), vec!["3", "2", "1"])]
#[case::desc_then_asc(json!([{ "a": "desc" }, { "b": "asc" }]), vec!["1", "2", "3"])]
#[case::mixed_grammar(json!(["a", { "b": { "order": "desc" } }]), vec!["3", "2", "1"])]
async fn test_sort_multi_field(
    scope: &TestScope,
    #[case] sort: Value,
    #[case] expected: Vec<&str>,
) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "a": 1, "b": 1 })),
            ("2", json!({ "a": 1, "b": 2 })),
            ("3", json!({ "a": 0, "b": 9 })),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "sort": sort }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), expected, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sort_multi_field_doc_missing_all_fields_sorts_last(scope: &TestScope) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "a": 1, "b": 1 })),
            ("2", json!({ "title": "no a, no b" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "a": "asc" }, { "b": "asc" }]
        }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), vec!["1", "2"], "{body}");
}

// ES retains docs missing the sort field and sorts them last in both
// directions (`missing: _last` default) — verified against ES serverless.
#[rstest_ctx(TestScope)]
#[case::asc(json!([{ "n": "asc" }]), vec!["3", "1", "2"])]
#[case::desc(json!([{ "n": "desc" }]), vec!["1", "3", "2"])]
async fn test_sort_single_field_missing_field_sorts_last(
    scope: &TestScope,
    #[case] sort: Value,
    #[case] expected: Vec<&str>,
) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "n": 2 })),
            ("2", json!({ "title": "no n here" })),
            ("3", json!({ "n": 1 })),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "sort": sort }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), expected, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sort_multi_field_missing_secondary_sorts_last_in_tie_group(scope: &TestScope) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "a": 1, "b": 1 })),
            ("2", json!({ "a": 1 })),
            ("3", json!({ "a": 0, "b": 5 })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "a": "asc" }, { "b": "asc" }]
        }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), vec!["3", "1", "2"], "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sort_multi_field_pagination_ties_broken_across_pages(scope: &TestScope) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "a": 1, "b": 4 })),
            ("2", json!({ "a": 1, "b": 3 })),
            ("3", json!({ "a": 0, "b": 9 })),
            ("4", json!({ "a": 2, "b": 0 })),
        ])
        .await;

    // Full order by [a asc, b asc]: 3, 2, 1, 4 — the page boundary splits
    // the a=1 tie group.
    let body = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "a": "asc" }, { "b": "asc" }],
            "from": 1,
            "size": 2,
        }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids(), vec!["2", "1"], "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_empty_sort_is_noop(scope: &TestScope) {
    scope.create().await;

    scope
        .index_docs([("1", json!({ "n": 1 })), ("2", json!({ "n": 2 }))])
        .await;

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "sort": [] }))
        .await
        .expect("search should succeed");
    assert_eq!(body.hit_ids().len(), 2, "{body}");
    // No sort means relevance scoring stays on and hits carry no sort key.
    assert!(!body.all_scores_null(), "{body}");
    assert!(body.max_score() > 0.0, "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_sort_values_in_hits(scope: &TestScope) {
    scope.create().await;

    scope
        .index_docs([
            ("1", json!({ "a": 1, "b": 1 })),
            ("2", json!({ "a": 1, "b": 2 })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match_all": {} },
            "sort": [{ "a": "asc" }, { "b": "desc" }]
        }))
        .await
        .expect("search should succeed");

    let hits = body["hits"]["hits"].as_array().unwrap();
    assert_eq!(hits[0]["_id"], "2", "{body}");
    assert_eq!(hits[0]["sort"], json!([1, 2]), "{body}");
    assert_eq!(hits[1]["_id"], "1", "{body}");
    assert_eq!(hits[1]["sort"], json!([1, 1]), "{body}");

    // Without a sort, hits must not carry a sort key.
    let body = scope
        .search(json!({ "query": { "match_all": {} } }))
        .await
        .expect("search should succeed");
    let hits = body["hits"]["hits"].as_array().unwrap();
    assert!(hits.iter().all(|h| h.get("sort").is_none()), "{body}");
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_match_standalone_scores_by_relevance_descending(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "text" }, "body": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            (
                "1",
                json!({ "title": "cats are great pets", "body": "cats cats love cats" }),
            ),
            (
                "2",
                json!({ "title": "dogs are loyal", "body": "cats appear once here" }),
            ),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "match": { "body": "cats" } } }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["1", "2"],
        "doc 1 mentions \"cats\" more and should rank first by default: {body}"
    );
    let score_1 = body.score("1");
    let score_2 = body.score("2");
    assert!(score_1 > score_2 && score_2 > 0.0);
    assert!((body.max_score() - score_1).abs() < 1e-6);
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_multi_match_standalone_aggregates_score_across_fields(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "text" }, "body": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            (
                "both",
                json!({ "title": "cats are great pets", "body": "many people love cats" }),
            ),
            (
                "one",
                json!({ "title": "dogs are loyal", "body": "cats appear once here" }),
            ),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "multi_match": { "query": "cats", "fields": ["title", "body"] } }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["both", "one"],
        "matching in both fields should score higher than matching in one: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_match_standalone_respects_explicit_sort(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "body": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "n": 2, "body": "cats cats cats" })),
            ("2", json!({ "n": 1, "body": "cats" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "sort": "n",
            "track_scores": true
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["2", "1"],
        "explicit sort should override the default relevance ordering: {body}"
    );
    assert!(body.score("1") > body.score("2"));
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bool_optional_should_with_sort_and_track_scores_reports_bm25(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "body": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "n": 2, "body": "cats cats cats" })),
            ("2", json!({ "n": 1, "body": "cats" })),
            ("3", json!({ "n": 3, "body": "dogs" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "filter": [{ "exists": { "field": "body" } }],
                    "should": [{ "match": { "body": "cats" } }]
                }
            },
            "sort": "n",
            "track_scores": true
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["2", "1", "3"],
        "explicit sort should order the hits: {body}"
    );
    assert!(body.score("1") > body.score("3") && body.score("2") > body.score("3"));
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_match_standalone_explicit_sort_without_track_scores_has_null_score(
    scope: &TestScope,
) {
    scope
        .create_with_properties(json!({ "body": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "n": 2, "body": "cats cats cats" })),
            ("2", json!({ "n": 1, "body": "cats" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "sort": "n"
        }))
        .await
        .expect("search should succeed");

    assert!(body.all_scores_null());
    assert!(body.max_score_is_null());
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_term_standalone_has_query_context_score(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": { "term": { "genre": "fantasy" } },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 3, "{body}");
    let score = body.score("1");
    assert!(score > 0.0 && (score - 1.0).abs() > 1e-6);
    assert!((body.score("2") - score).abs() < 1e-6);
    assert!((body.score("4") - score).abs() < 1e-6);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_range_standalone_has_query_context_constant_score(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": { "range": { "n": { "gte": 2 } } },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 3, "{body}");
    assert_eq!(body.score("1"), 1.0, "{body}");
    assert_eq!(body.score("3"), 1.0, "{body}");
    assert_eq!(body.score("4"), 1.0, "{body}");
}

#[rstest_ctx(TestScope)]
#[case::term(json!({ "term": { "genre": "fantasy" } }), vec!["1", "2", "4"])]
#[case::range(json!({ "range": { "n": { "gte": 2 } } }), vec!["1", "3", "4"])]
async fn test_bool_filter_scalar_clause_has_zero_score(
    scope: &TestScope,
    #[case] clause: Value,
    #[case] ids: Vec<&str>,
) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": { "bool": { "filter": [clause] } },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), ids.len(), "{body}");
    for id in ids {
        assert_eq!(body.score(id), 0.0, "{body}");
    }
}

#[test_context(TestScope)]
#[tokio::test]
async fn dev_bool_must_scalar_term_has_query_context_score(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "must": [{ "term": { "genre": "fantasy" } }]
                }
            }
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 3, "{body}");
    let score = body.score("1");
    assert!(score > 0.0 && (score - 1.0).abs() > 1e-6);
    assert!((body.score("2") - score).abs() < 1e-6);
    assert!((body.score("4") - score).abs() < 1e-6);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bool_must_not_text_excludes_without_contributing_score(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let without_must_not = scope
        .search(json!({
            "query": { "match": { "body": "cats" } },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "must": [{ "match": { "body": "cats" } }],
                    "must_not": [{ "match": { "title": "alpha" } }]
                }
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 2, "{body}");
    assert!(!body.hit_ids().contains(&"1".to_string()));
    assert!((body.score("2") - without_must_not.score("2")).abs() < 1e-6);
    assert!((body.score("4") - without_must_not.score("4")).abs() < 1e-6);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bool_should_text_only_scores_and_requires_a_should_match(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "should": [
                        { "match": { "body": "cats" } },
                        { "match": { "title": "alpha" } }
                    ]
                }
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 4, "{body}");
    assert!(body.score("1") > body.score("2") && body.score("1") > body.score("3"));
    assert!(body.score("2") > 0.0 && body.score("3") > 0.0);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bool_must_text_and_should_text_add_scores(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "must": [{ "match": { "body": "cats" } }],
                    "should": [{ "match": { "title": "alpha" } }]
                }
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 3, "{body}");
    assert!(body.score("1") > body.score("2") && body.score("1") > body.score("4"));
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bool_filter_with_should_text_scores_optional_matches_without_gating(
    scope: &TestScope,
) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "filter": [{ "term": { "genre": "fantasy" } }],
                    "should": [{ "match": { "title": "alpha" } }]
                }
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 3, "{body}");
    assert!(body.score("1") > 0.0);
    assert_eq!(
        (body.score("2"), body.score("4")),
        (0.0, 0.0),
        "docs that satisfy the filter but miss optional should text should still return with no text score: {body}"
    );
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_bool_nested_should_inside_must_scores_matching_children(scope: &TestScope) {
    setup_bool_scoring_docs(scope).await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "must": [{
                        "bool": {
                            "should": [
                                { "match": { "body": "cats" } },
                                { "match": { "title": "alpha" } }
                            ]
                        }
                    }]
                }
            },
            "size": 10
        }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 4, "{body}");
    assert!(body.score("1") > body.score("2") && body.score("1") > body.score("3"));
    assert!(body.score("2") > 0.0 && body.score("3") > 0.0 && body.score("4") > 0.0);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_match_nested_in_bool_scores_and_applies_filter(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "body": { "type": "text" },
            "genre": { "type": "keyword" }
        }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "body": "cats cats cats", "genre": "fantasy" })),
            ("2", json!({ "body": "cats", "genre": "fantasy" })),
            ("3", json!({ "body": "cats cats cats", "genre": "sci-fi" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "must": [{ "match": { "body": "cats" } }],
                    "filter": [{ "term": { "genre": "fantasy" } }]
                }
            }
        }))
        .await
        .expect("search should succeed");
    assert_eq!(
        body.hit_ids(),
        vec!["1", "2"],
        "scalar bool.filter should narrow candidates before BM25 ranking: {body}"
    );
    assert!(body.score("1") > body.score("2") && body.score("2") > 0.0);
}

#[rstest_ctx(TestScope)]
#[case::match_all(
    json!({ "match_all": {} }),
    json!({ "match_all": { "boost": 2.0 } })
)]
#[case::ids(
    json!({ "ids": { "values": ["1"] } }),
    json!({ "ids": { "values": ["1"], "boost": 2.0 } })
)]
#[case::match_query(
    json!({ "match": { "body": "cats" } }),
    json!({ "match": { "body": { "query": "cats", "boost": 2.0 } } })
)]
#[case::term(
    json!({ "term": { "genre": "fantasy" } }),
    json!({ "term": { "genre": { "value": "fantasy", "boost": 2.0 } } })
)]
#[case::multi_match(
    json!({ "multi_match": { "query": "cats", "fields": ["title", "body"] } }),
    json!({ "multi_match": { "query": "cats", "fields": ["title", "body"], "boost": 2.0 } })
)]
#[case::range(
    json!({ "range": { "n": { "gte": 2 } } }),
    json!({ "range": { "n": { "gte": 2, "boost": 2.0 } } })
)]
#[case::terms(
    json!({ "terms": { "genre": ["fantasy"] } }),
    json!({ "terms": { "genre": ["fantasy"], "boost": 2.0 } })
)]
#[case::bool(
    json!({ "bool": { "must": [{ "match": { "body": "cats" } }] } }),
    json!({ "bool": { "must": [{ "match": { "body": "cats" } }], "boost": 2.0 } })
)]
async fn test_boost_multiplies_score(
    scope: &TestScope,
    #[case] unboosted_query: Value,
    #[case] boosted_query: Value,
) {
    setup_bool_scoring_docs(scope).await;

    let unboosted = scope
        .search(json!({ "query": unboosted_query, "size": 10 }))
        .await
        .expect("search should succeed");

    let boosted = scope
        .search(json!({ "query": boosted_query, "size": 10 }))
        .await
        .expect("search should succeed");

    assert!(boosted.score("1") > unboosted.score("1") * 1.5);
}

#[test_context(TestScope)]
#[tokio::test]
async fn test_match_nested_in_bool_filter_does_not_score(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "body": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "body": "cats cats cats" })),
            ("2", json!({ "body": "cats" })),
        ])
        .await;

    let body = scope
        .search(json!({
            "query": {
                "bool": {
                    "filter": [{ "match": { "body": "cats" } }]
                }
            }
        }))
        .await
        .expect("search should succeed");

    let ids = body.hit_ids();
    assert_eq!(ids.len(), 2, "{body}");
    assert!(ids.contains(&"1".to_string()));
    assert!(ids.contains(&"2".to_string()));
    assert_eq!(
        (body.score("1"), body.score("2")),
        (0.0, 0.0),
        "bool.filter match should narrow without contributing BM25 score: {body}"
    );
}

#[rstest_ctx(BooksContext)]
#[case::dev_full_document(
    json!(null),
    Some(json!({
        "title": "The Hobbit",
        "author": "Tolkien",
        "published_year": 1937,
        "rating": 4.3,
        "genre": "fantasy",
        "in_print": true,
        "tags": ["fantasy", "adventure", "tolkien"],
        "embedding": [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "token_embeddings": [[0.5, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
    }))
)]
#[case::false_omits_key(json!(false), None)]
#[case::star_include_all_fields(
    json!(["*"]),
    Some(json!({
        "title": "The Hobbit",
        "author": "Tolkien",
        "published_year": 1937,
        "rating": 4.3,
        "genre": "fantasy",
        "in_print": true,
        "tags": ["fantasy", "adventure", "tolkien"],
        "embedding": [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "token_embeddings": [[0.5, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]],
    }))
)]
#[case::includes_filters_fields(
    json!(["title", "genre"]),
    Some(json!({ "title": "The Hobbit", "genre": "fantasy" }))
)]
#[case::string_include_single_field(
    json!("title"),
    Some(json!({ "title": "The Hobbit" }))
)]
#[case::excludes_drops_fields(
    json!({ "excludes": ["embedding", "token_embeddings", "tags"] }),
    Some(json!({
        "title": "The Hobbit",
        "author": "Tolkien",
        "published_year": 1937,
        "rating": 4.3,
        "genre": "fantasy",
        "in_print": true,
    }))
)]
async fn test_search_source_filtering(
    books: &BooksContext,
    #[case] source: Value,
    #[case] expected: Option<Value>,
) {
    let mut body = json!({ "query": { "term": { "genre": "fantasy" } }, "size": 10 });
    if !source.is_null() {
        body["_source"] = source;
    }

    let body = books.search(body).await.expect("search should succeed");

    match expected {
        Some(expected) => assert_eq!(body.source("hobbit"), &expected, "{body}"),
        None => assert!(body.all_source_omitted(), "{body}"),
    }
}

#[rstest_ctx(TestScope)]
#[case::wildcard(
    json!(["meta.*"]),
    json!({ "meta": { "author": "ada", "year": 2024 } })
)]
#[case::exact_leaf_path(
    json!(["meta.author"]),
    json!({ "meta": { "author": "ada" } })
)]
#[case::mixed_top_level_and_nested(
    json!(["title", "meta.author"]),
    json!({ "title": "hello", "meta": { "author": "ada" } })
)]
async fn test_search_source_nested_field_paths(
    scope: &TestScope,
    #[case] source: Value,
    #[case] expected: Value,
) {
    scope.create().await;
    scope
        .index_doc(
            "1",
            json!({ "title": "hello", "meta": { "author": "ada", "year": 2024 } }),
        )
        .await;

    let body = scope
        .search(json!({ "query": { "match_all": {} }, "_source": source }))
        .await
        .expect("search should succeed");

    assert_eq!(body.source("1"), &expected, "{body}");
}
