mod common;

use common::TestScope;
use elasticsearch::http::StatusCode;
use serde_json::json;
use test_context::test_context;
use test_macros::rstest_ctx;

// A keyword field indexes the whole value as one verbatim term, so `match`
// only matches the entire value — not a partial token, and case-sensitively.
#[test_context(TestScope)]
#[tokio::test]
async fn test_match_on_keyword_is_exact_and_verbatim(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "tag": { "type": "keyword" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "tag": "New York City" })),
            ("2", json!({ "tag": "CamelCase" })),
        ])
        .await;

    // Whole value matches.
    assert_eq!(
        scope
            .search_ids(json!({ "match": { "tag": "New York City" } }))
            .await,
        vec!["1"],
        "match on the full keyword value should hit"
    );

    // Partial token does not match (ES: keyword is not tokenized).
    assert_eq!(
        scope
            .search_ids(json!({ "match": { "tag": "York" } }))
            .await,
        Vec::<String>::new(),
        "match on a partial token of a keyword value must not hit"
    );

    // Case-sensitive: keyword is not lowercased.
    assert_eq!(
        scope
            .search_ids(json!({ "match": { "tag": "camelcase" } }))
            .await,
        Vec::<String>::new(),
        "match on a keyword value must be case-sensitive"
    );
}

// term/terms on a keyword field are exact and match the whole value.
#[test_context(TestScope)]
#[tokio::test]
async fn test_term_on_keyword_is_exact(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "tag": { "type": "keyword" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "tag": "New York City" })),
            ("2", json!({ "tag": "Boston" })),
        ])
        .await;

    assert_eq!(
        scope
            .search_ids(json!({ "term": { "tag": "New York City" } }))
            .await,
        vec!["1"]
    );
    assert_eq!(
        scope.search_ids(json!({ "term": { "tag": "York" } })).await,
        Vec::<String>::new(),
        "term on a keyword field must match the whole value, not a token"
    );
}

// A keyword `term` in query context carries a real (IDF-based) score, not a
// flat constant — the whole value is a scored verbatim text term.
#[test_context(TestScope)]
#[tokio::test]
async fn test_term_on_keyword_has_idf_score(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "genre": { "type": "keyword" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "genre": "fantasy" })),
            ("2", json!({ "genre": "fantasy" })),
            ("3", json!({ "genre": "fiction" })),
        ])
        .await;

    let body = scope
        .search(json!({ "query": { "term": { "genre": "fantasy" } }, "size": 10 }))
        .await
        .expect("search should succeed");

    assert_eq!(body.hit_ids().len(), 2, "{body}");
    let score = body.score("1");
    assert!(
        score > 0.0 && (score - 1.0).abs() > 1e-6,
        "keyword term should score by IDF, not a flat 1.0: {body}"
    );
    assert!(
        (body.score("2") - score).abs() < 1e-6,
        "equal-frequency terms score equally: {body}"
    );
}

// `term` on an analyzed text field matches an indexed token (ES semantics),
// unlike the exact scalar comparison used for keyword fields.
#[test_context(TestScope)]
#[tokio::test]
async fn test_term_on_text_matches_token(scope: &TestScope) {
    scope
        .create_with_properties(json!({ "title": { "type": "text" } }))
        .await;

    scope
        .index_docs([
            ("1", json!({ "title": "the quick fox" })),
            ("2", json!({ "title": "lazy dogs" })),
        ])
        .await;

    assert_eq!(
        scope
            .search_ids(json!({ "term": { "title": "fox" } }))
            .await,
        vec!["1"],
        "term on a text field should match an indexed token"
    );
}

// Sort and terms-agg on an analyzed text field are rejected (ES: fielddata is
// disabled), while the same operations on a keyword field are allowed.
#[test_context(TestScope)]
#[tokio::test]
async fn test_sort_and_agg_on_text_rejected(scope: &TestScope) {
    scope
        .create_with_properties(json!({
            "title": { "type": "text" },
            "tag": { "type": "keyword" }
        }))
        .await;

    scope
        .index_docs([("1", json!({ "title": "hello world", "tag": "a" }))])
        .await;

    let sort_err = scope
        .search(json!({ "query": { "match_all": {} }, "sort": [{ "title": "asc" }] }))
        .await
        .unwrap_err();
    assert_eq!(
        sort_err.status_code(),
        StatusCode::BAD_REQUEST,
        "sort on an analyzed text field must be rejected"
    );

    let agg_err = scope
        .search(json!({ "size": 0, "aggs": { "t": { "terms": { "field": "title" } } } }))
        .await
        .unwrap_err();
    assert_eq!(
        agg_err.status_code(),
        StatusCode::BAD_REQUEST,
        "terms agg on an analyzed text field must be rejected"
    );

    // Same operations on a keyword field are fine.
    scope
        .search(json!({ "query": { "match_all": {} }, "sort": [{ "tag": "asc" }] }))
        .await
        .expect("sort on a keyword field should succeed");
    scope
        .search(json!({ "size": 0, "aggs": { "t": { "terms": { "field": "tag" } } } }))
        .await
        .expect("terms agg on a keyword field should succeed");
}

// GET /_mapping reports keyword fields as keyword (not analyzed text).
#[rstest_ctx(TestScope)]
#[case::keyword("keyword", "keyword")]
#[case::text("text", "text")]
async fn test_mapping_reports_declared_string_type(
    scope: &TestScope,
    #[case] declared: &str,
    #[case] expected: &str,
) {
    scope
        .create_with_properties(json!({ "f": { "type": declared } }))
        .await;

    let res = scope
        .client
        .es()
        .indices()
        .get_mapping(elasticsearch::indices::IndicesGetMappingParts::Index(&[
            &scope.name,
        ]))
        .send()
        .await
        .expect("get mapping");
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(
        body[&scope.name]["mappings"]["properties"]["f"]["type"], expected,
        "{body}"
    );
}
