use std::collections::HashSet;

use rstest::rstest;

use topk_rs::{doc, proto::v1::data::Document, proto::v1::data::Value};

mod common;
use common::{BooksContext, Scope, ids};

#[rstest]
#[case::single_field(
    "SELECT title FROM {{table}} WHERE _id = 'hobbit'",
    vec![doc!("title" => "The Hobbit")],
)]
#[case::multi_field(
    "SELECT title, author, published_year FROM {{table}} WHERE _id = 'hobbit'",
    vec![doc!("title" => "The Hobbit", "author" => "Tolkien", "published_year" => 1937_i64)],
)]
#[case::alias_expr(
    "SELECT published_year, published_year + 1 AS next_year FROM {{table}} WHERE _id = 'hobbit'",
    vec![doc!("published_year" => 1937_i64, "next_year" => 1938_i64)],
)]
#[case::arithmetic_expr(
    "SELECT _id, rating, rating * 2 AS double_rating FROM {{table}} WHERE _id = 'lotr'",
    vec![doc!("_id" => "lotr", "rating" => 4.5_f64, "double_rating" => 9.0_f64)],
)]
#[case::function_expr(
    "SELECT _id, ABS(published_year - 2000) AS distance FROM {{table}} WHERE _id = 'pride'",
    vec![doc!("_id" => "pride", "distance" => 187_i64)],
)]
#[case::ordered_expr(
    "SELECT _id, ABS(published_year - 1953) AS d FROM {{table}} ORDER BY ABS(published_year - 1953) ASC LIMIT 1",
    vec![doc!("_id" => "lotr", "d" => 1_i64)],
)]
#[case::case_searched(
    "SELECT _id, CASE WHEN rating >= 4.5 THEN 'great' ELSE 'good' END AS tier FROM {{table}} WHERE _id = 'lotr'",
    vec![doc!("_id" => "lotr", "tier" => "great")],
)]
#[case::case_searched_else_branch(
    "SELECT _id, CASE WHEN rating >= 4.5 THEN 'great' ELSE 'good' END AS tier FROM {{table}} WHERE _id = 'gatsby'",
    vec![doc!("_id" => "gatsby", "tier" => "good")],
)]
#[case::case_searched_multi_when(
    "SELECT _id, CASE WHEN rating >= 4.5 THEN 'great' WHEN rating >= 4.0 THEN 'good' ELSE 'ok' END AS tier FROM {{table}} WHERE _id = 'mockingbird'",
    vec![doc!("_id" => "mockingbird", "tier" => "good")],
)]
#[case::case_simple(
    "SELECT _id, CASE genre WHEN 'fantasy' THEN 'spec_fic' ELSE 'other' END AS category FROM {{table}} WHERE _id = 'hobbit'",
    vec![doc!("_id" => "hobbit", "category" => "spec_fic")],
)]
#[case::case_no_else(
    "SELECT _id, CASE WHEN rating >= 4.5 THEN 'great' END AS tier FROM {{table}} WHERE _id = 'gatsby'",
    vec![doc!("_id" => "gatsby", "tier" => Value::null())],
)]
#[tokio::test]
async fn select(#[case] query: &str, #[case] expected: Vec<Document>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();

    assert_eq!(rows, expected);
}

#[rstest]
#[case::text_eq("SELECT _id FROM {{table}} WHERE author = 'Tolkien'", ids!["hobbit", "lotr"])]
#[case::int_gt("SELECT _id FROM {{table}} WHERE published_year > 1950", ids!["mockingbird", "catcher", "lotr", "harry", "alchemist"])]
#[case::bool_eq("SELECT _id FROM {{table}} WHERE in_print = false", ids!["catcher", "moby"])]
#[case::int_lt("SELECT _id FROM {{table}} WHERE published_year < 1925", ids!["pride", "moby"])]
#[case::int_lte("SELECT _id FROM {{table}} WHERE published_year <= 1925", ids!["pride", "moby", "gatsby"])]
#[case::single_match("SELECT _id FROM {{table}} WHERE published_year > 1990", ids!["harry"])]
#[case::float_gte("SELECT _id FROM {{table}} WHERE rating >= 4.5", ids!["lotr", "harry"])]
#[case::not_equal("SELECT _id FROM {{table}} WHERE genre <> 'fiction' AND genre <> 'fantasy'", ids!["nineteen_eighty_four", "pride", "moby"])]
#[case::unary_not("SELECT _id FROM {{table}} WHERE NOT genre = 'fiction'", ids!["nineteen_eighty_four", "pride", "hobbit", "lotr", "harry", "moby"])]
#[case::or_expr("SELECT _id FROM {{table}} WHERE genre = 'fantasy' OR genre = 'romance'", ids!["hobbit", "lotr", "harry", "pride"])]
#[case::not_null("SELECT _id FROM {{table}} WHERE rating IS NOT NULL", ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "catcher", "hobbit", "lotr", "harry", "alchemist", "moby"])]
#[case::is_null("SELECT _id FROM {{table}} WHERE rating IS NULL", ids![])]
#[tokio::test]
async fn where_logical(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::and_expr(
    "SELECT _id FROM {{table}} WHERE author = 'Tolkien' AND published_year > 1940",
    ids!["lotr"],
)]
#[case::or_expr(
    "SELECT _id FROM {{table}} WHERE author = 'Tolkien' OR author = 'Rowling'",
    ids!["hobbit", "lotr", "harry"],
)]
#[case::nested_expr(
    "SELECT _id FROM {{table}} WHERE (genre = 'fantasy' OR genre = 'fiction') AND published_year > 1950",
    ids!["mockingbird", "catcher", "lotr", "harry", "alchemist"],
)]
#[case::not_group(
    "SELECT _id FROM {{table}} WHERE NOT (genre = 'fiction' OR genre = 'adventure')",
    ids!["nineteen_eighty_four", "pride", "hobbit", "lotr", "harry"],
)]
#[case::triple_and(
    "SELECT _id FROM {{table}} WHERE rating >= 4.0 AND published_year >= 1950 AND in_print = true",
    ids!["mockingbird", "lotr", "harry"],
)]
#[case::multi_and(
    "SELECT _id FROM {{table}} WHERE rating >= 4.0 AND published_year >= 1900 AND in_print = true AND author <> 'Tolkien' AND genre <> 'romance'",
    ids!["mockingbird", "nineteen_eighty_four", "harry"],
)]
#[case::multi_or(
    "SELECT _id FROM {{table}} WHERE author = 'Lee' OR author = 'Salinger' OR author = 'Melville'",
    ids!["mockingbird", "catcher", "moby"],
)]
#[case::nested_parens(
    "SELECT _id FROM {{table}} WHERE ((((rating > 4.0)) AND (genre = 'fantasy')))",
    ids!["hobbit", "lotr", "harry"],
)]
#[case::precedence(
    "SELECT _id FROM {{table}} WHERE author = 'Tolkien' OR rating >= 4.5 AND in_print = true",
    ids!["hobbit", "lotr", "harry"],
)]
#[tokio::test]
async fn where_nary(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::inclusive_range(
    "SELECT _id FROM {{table}} WHERE published_year BETWEEN 1940 AND 1960",
    ids!["mockingbird", "nineteen_eighty_four", "catcher", "lotr"],
)]
#[case::negated_range(
    "SELECT _id FROM {{table}} WHERE published_year NOT BETWEEN 1900 AND 2000",
    ids!["pride", "moby"],
)]
#[case::float_range(
    "SELECT _id FROM {{table}} WHERE rating BETWEEN 4.0 AND 4.4",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "hobbit"],
)]
#[case::single_value(
    "SELECT _id FROM {{table}} WHERE published_year BETWEEN 1851 AND 1851",
    ids!["moby"],
)]
#[case::not_between(
    "SELECT _id FROM {{table}} WHERE published_year NOT BETWEEN 1925 AND 1960",
    ids!["pride", "moby", "alchemist", "harry"],
)]
#[tokio::test]
async fn where_between(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::text_list(
    "SELECT _id FROM {{table}} WHERE genre IN ('fantasy','romance')",
    ids!["hobbit", "lotr", "harry", "pride"],
)]
#[case::int_list(
    "SELECT _id FROM {{table}} WHERE published_year IN (1949, 1925, 1851)",
    ids!["nineteen_eighty_four", "gatsby", "moby"],
)]
#[case::not_in(
    "SELECT _id FROM {{table}} WHERE genre NOT IN ('fiction','adventure')",
    ids!["nineteen_eighty_four", "pride", "hobbit", "lotr", "harry"],
)]
#[case::single_item(
    "SELECT _id FROM {{table}} WHERE author IN ('Tolkien')",
    ids!["hobbit", "lotr"],
)]
#[case::float_list(
    "SELECT _id FROM {{table}} WHERE rating IN (4.5, 3.5)",
    ids!["lotr", "harry", "moby"],
)]
#[case::text_not_in(
    "SELECT _id FROM {{table}} WHERE author NOT IN ('Tolkien')",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "catcher", "harry", "alchemist", "moby"],
)]
#[tokio::test]
async fn where_in(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::prefix("WHERE _id LIKE 'h%'", ids!["hobbit", "harry"])]
#[case::contains("WHERE _id LIKE '%ob%'", ids!["hobbit", "moby"])]
#[case::exact("WHERE _id LIKE 'gatsby'", ids!["gatsby"])]
#[case::field_prefix("WHERE author LIKE 'T%'", ids!["hobbit", "lotr"])]
#[case::field_contains("WHERE title LIKE '%Lord%'", ids!["lotr"])]
#[case::field_exact("WHERE genre LIKE 'romance'", ids!["pride"])]
#[tokio::test]
async fn where_like(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(format!("SELECT _id FROM {{{{table}}}} {query}"))
            .await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::list_member("WHERE contains(tags, 'tolkien')", ids!["hobbit", "lotr"])]
#[case::string_substring("WHERE contains(title, 'Lord')", ids!["lotr"])]
#[tokio::test]
async fn where_contains(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(format!("SELECT _id FROM {{{{table}}}} {query}"))
            .await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::prefix("SELECT _id FROM {{table}} WHERE _id ~ '^h'", ids!["hobbit", "harry"])]
#[case::case_insensitive("SELECT _id FROM {{table}} WHERE author ~ '(?i)tolkien'", ids!["hobbit", "lotr"])]
#[case::suffix("SELECT _id FROM {{table}} WHERE _id ~ 'r$'", ids!["nineteen_eighty_four", "catcher", "lotr"])]
#[case::alternation("SELECT _id FROM {{table}} WHERE _id ~ '^(harry|hobbit)$'", ids!["harry", "hobbit"])]
#[case::char_class("SELECT _id FROM {{table}} WHERE _id ~ '^[a-c]'", ids!["alchemist", "catcher"])]
#[tokio::test]
async fn where_regex(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::add_expr(
    "SELECT _id FROM {{table}} WHERE published_year + 100 > 2050",
    ids!["mockingbird", "catcher", "lotr", "harry", "alchemist"],
)]
#[case::sub_expr(
    "SELECT _id FROM {{table}} WHERE published_year - 50 > 1900",
    ids!["mockingbird", "catcher", "lotr", "harry", "alchemist"],
)]
#[case::mul_expr("SELECT _id FROM {{table}} WHERE rating * 2 >= 9.0", ids!["lotr", "harry"])]
#[case::div_expr("SELECT _id FROM {{table}} WHERE rating / 2 > 2.2", ids!["lotr", "harry"])]
#[case::negative_expr(
    "SELECT _id FROM {{table}} WHERE published_year - 2000 < -50",
    ids!["pride", "moby", "gatsby", "hobbit", "nineteen_eighty_four"],
)]
#[case::case_expr(
    "SELECT _id FROM {{table}} \
     WHERE rating >= CASE WHEN genre = 'fantasy' THEN 4.5 ELSE 4.3 END",
    ids!["lotr", "harry", "mockingbird", "pride"],
)]
#[case::simple_case(
    "SELECT _id FROM {{table}} \
     WHERE 1 = CASE genre WHEN 'fiction' THEN 1 ELSE 0 END LIMIT 5",
    ids!["mockingbird", "gatsby", "catcher", "alchemist"],
)]
#[case::case_null_else(
    "SELECT _id FROM {{table}} \
     WHERE rating >= CASE WHEN genre = 'fantasy' THEN 4.5 END",
    ids!["lotr", "harry"],
)]
#[case::abs_expr(
    "SELECT _id FROM {{table}} WHERE ABS(published_year - 1950) < 5",
    ids!["nineteen_eighty_four", "catcher", "lotr"],
)]
#[case::abs_lte("SELECT _id FROM {{table}} WHERE ABS(published_year - 1953) <= 1", ids!["lotr"])]
#[case::nested_abs(
    "SELECT _id FROM {{table}} WHERE ABS(-5 + published_year - 1953) < 5",
    ids!["mockingbird", "lotr"],
)]
#[case::greatest("SELECT _id FROM {{table}} WHERE GREATEST(rating, 4.0) >= 4.4", ids!["lotr", "harry"])]
#[case::least(
    "SELECT _id FROM {{table}} WHERE LEAST(rating, 4.0) < 4.0",
    ids!["gatsby", "catcher", "alchemist", "moby"],
)]
#[case::sqrt("SELECT _id FROM {{table}} WHERE SQRT(rating) > 2.1", ids!["lotr", "harry"])]
#[case::sqrt_square("SELECT _id FROM {{table}} WHERE SQRT(SQUARE(rating)) >= 4.5", ids!["lotr", "harry"])]
#[case::square("SELECT _id FROM {{table}} WHERE SQUARE(rating) > 20.0", ids!["lotr", "harry"])]
#[case::natural_log("SELECT _id FROM {{table}} WHERE LN(rating) > 1.5", ids!["lotr", "harry"])]
#[case::exponential("SELECT _id FROM {{table}} WHERE EXP(rating) > 89.0", ids!["lotr", "harry"])]
#[case::coalesce("SELECT _id FROM {{table}} WHERE COALESCE(rating, 0) >= 4.5", ids!["lotr", "harry"])]
#[case::match_all("SELECT _id FROM {{table}} WHERE match_all(title, 'kill')", ids!["mockingbird"])]
#[case::match_any(
    "SELECT _id FROM {{table}} WHERE match_any(title, 'hobbit rings')",
    ids!["hobbit", "lotr"],
)]
#[case::boost_score(
    "SELECT _id, BOOST(rating, in_print = true, 1.5) AS score FROM {{table}} ORDER BY score DESC LIMIT 2",
    ids!["lotr", "harry"],
)]
#[tokio::test]
async fn where_expr(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();

    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::searched_case(
    "SELECT _id, \
            CASE WHEN rating >= 4.5 THEN 'top' \
                 WHEN rating >= 4.0 THEN 'good' \
                 WHEN rating >= 3.5 THEN 'ok' \
                 ELSE 'mid' END AS out \
     FROM {{table}} WHERE _id IN ('lotr', 'mockingbird', 'gatsby', 'moby')",
    vec![
        doc!("_id" => "lotr", "out" => "top"),
        doc!("_id" => "mockingbird", "out" => "good"),
        doc!("_id" => "gatsby", "out" => "ok"),
        doc!("_id" => "moby", "out" => "ok"),
    ],
)]
#[case::nested_case(
    "SELECT _id, \
            CASE WHEN in_print = true THEN \
                 CASE WHEN rating >= 4.5 THEN 'top' ELSE 'live' END \
                 ELSE 'oop' END AS out \
     FROM {{table}} WHERE _id IN ('lotr', 'moby', 'gatsby')",
    vec![
        doc!("_id" => "lotr", "out" => "top"),
        doc!("_id" => "gatsby", "out" => "live"),
        doc!("_id" => "moby", "out" => "oop"),
    ],
)]
#[tokio::test]
async fn case_projection(#[case] query: &str, #[case] expected: Vec<Document>) {
    let mut rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();

    rows.sort_by_key(|doc| doc.id().unwrap().to_string());
    let mut expected = expected;
    expected.sort_by_key(|doc| doc.id().unwrap().to_string());
    assert_eq!(rows, expected);
}

#[rstest]
#[case::all_rows("SELECT COUNT(*) FROM {{table}}", vec![doc!("_count" => 10_i64)])]
#[case::empty_filter("SELECT COUNT(*) FROM {{table}} WHERE author = 'Nobody'", vec![doc!("_count" => 0_i64)])]
#[case::bool_filter("SELECT COUNT(*) FROM {{table}} WHERE in_print = true", vec![doc!("_count" => 8_i64)])]
#[case::text_filter("SELECT COUNT(*) FROM {{table}} WHERE genre = 'fantasy'", vec![doc!("_count" => 3_i64)])]
#[case::compound_filter(
    "SELECT COUNT(*) FROM {{table}} WHERE rating >= 4.0 AND published_year >= 1950",
    vec![doc!("_count" => 3_i64)],
)]
#[case::aliased("SELECT COUNT(*) AS f FROM {{table}}", vec![doc!("f" => 10_i64)])]
#[tokio::test]
async fn count(#[case] query: &str, #[case] expected: Vec<Document>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(rows, expected);
}

#[rstest]
#[case::ascending(
    "SELECT _id FROM {{table}} ORDER BY published_year ASC LIMIT 3",
    vec!["pride", "moby", "gatsby"],
)]
#[case::descending(
    "SELECT _id FROM {{table}} ORDER BY published_year DESC LIMIT 3",
    vec!["harry", "alchemist", "mockingbird"],
)]
#[tokio::test]
async fn order_by(#[case] query: &str, #[case] expected: Vec<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    let actual = rows.iter().map(|row| row.id().unwrap()).collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[rstest]
#[case::descending(
    "SELECT _id FROM {{table}} ORDER BY rating DESC LIMIT 2",
    ids!["lotr", "harry"],
)]
#[case::ascending("SELECT _id FROM {{table}} ORDER BY rating ASC LIMIT 1", ids!["moby"])]
#[tokio::test]
async fn order_by_unordered(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::single_row("SELECT _id FROM {{table}} LIMIT 1", 1)]
#[case::partial_page("SELECT _id FROM {{table}} LIMIT 3", 3)]
#[case::oversized("SELECT _id FROM {{table}} LIMIT 1000", 10)]
#[tokio::test]
async fn limit_caps_results(#[case] query: &str, #[case] expected_len: usize) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(rows.len(), expected_len);
}

#[rstest]
#[case::default_args(
    "SELECT _id, bm25_score() AS score FROM {{table}} WHERE match_any(title, 'rings') ORDER BY score DESC LIMIT 3",
    ids!["lotr"],
)]
#[case::custom_args(
    "SELECT _id, bm25_score(0.75, 1.2) AS score FROM {{table}} WHERE match_any(title, 'rings') ORDER BY score DESC LIMIT 3",
    ids!["lotr"],
)]
#[tokio::test]
async fn bm25_search(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[tokio::test]
async fn keyword_or_logical_filter() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(
            "SELECT _id FROM {{table}} WHERE match_any(title, 'rings') OR published_year = 1813",
        )
        .await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), ids!["lotr", "pride"]);
}

#[rstest]
#[case::bytes_null(
    "SELECT _id FROM {{table}} WHERE checksum IS NULL",
    ids!["lotr", "harry", "catcher", "alchemist", "moby"],
)]
#[case::bytes_present(
    "SELECT _id FROM {{table}} WHERE checksum IS NOT NULL",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "hobbit"],
)]
#[case::sparse_null(
    "SELECT _id FROM {{table}} WHERE sparse_emb IS NULL",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "catcher", "alchemist", "moby"],
)]
#[case::sparse_present(
    "SELECT _id FROM {{table}} WHERE sparse_emb IS NOT NULL",
    ids!["hobbit", "lotr", "harry"],
)]
#[case::nested_null(
    "SELECT _id FROM {{table}} WHERE metadata.publisher IS NULL",
    ids!["catcher", "hobbit", "lotr", "harry", "alchemist", "moby"],
)]
#[case::nested_present(
    "SELECT _id FROM {{table}} WHERE metadata.publisher IS NOT NULL",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby"],
)]
#[case::list_null("SELECT _id FROM {{table}} WHERE tags IS NULL", ids![])]
#[case::list_present(
    "SELECT _id FROM {{table}} WHERE tags IS NOT NULL",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "catcher", "hobbit", "lotr", "harry", "alchemist", "moby"],
)]
#[tokio::test]
async fn where_complex_null(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[tokio::test]
async fn dotted_field_output_name() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT metadata.publisher FROM {{table}} WHERE _id = 'mockingbird'")
            .await
    })
    .await
    .unwrap();

    let row = rows.first().expect("expected one row");
    assert_eq!(
        row.fields
            .get("metadata.publisher")
            .and_then(Value::as_string),
        Some("HarperCollins")
    );
    assert!(row.fields.get("publisher").is_none());
}

#[tokio::test]
async fn select_complex_fields() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT _id, tags, checksum, metadata.publisher AS publisher FROM {{table}} WHERE _id = 'mockingbird'")
            .await
    })
    .await
    .unwrap();
    let row = rows.first().expect("expected one row");
    assert!(row.fields.get("tags").is_some(), "tags should be non-null");
    assert!(
        row.fields.get("checksum").is_some(),
        "checksum should be non-null"
    );
    assert!(
        row.fields.get("publisher").is_some(),
        "publisher should be non-null"
    );
}

#[tokio::test]
async fn output_casts() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(
            "SELECT \
                title AS title_json, \
                title::text AS title_text, \
                published_year AS year_json, \
                published_year::bigint AS year_bigint, \
                rating AS rating_json, \
                rating::float8 AS rating_float, \
                in_print AS print_json, \
                in_print::boolean AS print_bool, \
                checksum AS checksum_json, \
                checksum::bytea AS checksum_bytea, \
                tags AS tags_json \
             FROM {{table}} WHERE _id = 'hobbit'",
        )
        .await
    })
    .await
    .unwrap();

    let row = rows.first().expect("expected one row");
    assert_eq!(
        row.fields.get("title_json").and_then(Value::as_string),
        Some("The Hobbit")
    );
    assert_eq!(
        row.fields.get("title_text").and_then(Value::as_string),
        Some("The Hobbit")
    );
    assert_eq!(
        row.fields.get("year_json").and_then(Value::as_i64),
        Some(1937)
    );
    assert_eq!(
        row.fields.get("year_bigint").and_then(Value::as_i64),
        Some(1937)
    );
    assert_eq!(
        row.fields.get("rating_json").and_then(Value::as_f64),
        Some(4.3)
    );
    assert_eq!(
        row.fields.get("rating_float").and_then(Value::as_f64),
        Some(4.3)
    );
    assert_eq!(
        row.fields.get("print_json").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        row.fields.get("print_bool").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        row.fields.get("checksum_json").and_then(Value::as_binary),
        Some([0x9a, 0xbc, 0xde, 0xf0].as_slice())
    );
    assert_eq!(
        row.fields.get("checksum_bytea").and_then(Value::as_binary),
        Some([0x9a, 0xbc, 0xde, 0xf0].as_slice())
    );
    assert_eq!(
        row.fields.get("tags_json").and_then(Value::as_string),
        Some("[\"fantasy\",\"adventure\",\"tolkien\"]")
    );
}

#[tokio::test]
async fn cast_without_alias() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT title::text FROM {{table}} WHERE _id = 'hobbit'")
            .await
    })
    .await
    .unwrap();

    let row = rows.first().expect("expected one row");
    assert_eq!(
        row.fields.get("title").and_then(Value::as_string),
        Some("The Hobbit")
    );
}

#[rstest]
#[case::dense_vector(
    "SELECT _id, vector_distance(embedding, f32_vector(ARRAY[1, 0, 0, 0])) AS score \
     FROM {{table}} ORDER BY score DESC LIMIT 3",
    ids!["hobbit", "lotr", "harry"],
)]
#[case::sparse_vector(
    "SELECT _id, vector_distance(sparse_emb, f32_sparse_vector(ARRAY[0], ARRAY[1.0])) AS score \
     FROM {{table}} WHERE sparse_emb IS NOT NULL ORDER BY score DESC LIMIT 2",
    ids!["hobbit", "harry"],
)]
#[case::multi_vector(
    "SELECT _id, multi_vector_distance(multi_emb, f32_matrix(ARRAY[ARRAY[1, 0, 0, 0]]), 10) AS score \
     FROM {{table}} ORDER BY score DESC LIMIT 3",
    ids!["hobbit", "lotr", "harry"],
)]
#[tokio::test]
async fn vector_distance_search(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();

    assert_eq!(ids(&rows), expected);
    let _: f32 = rows
        .first()
        .expect("expected one row")
        .fields
        .get("score")
        .and_then(Value::as_f32)
        .expect("score should be present");
}

#[tokio::test]
async fn semantic_similarity_search() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(
            "SELECT _id, semantic_similarity(bio, 'fantasy quest') AS sim \
             FROM {{table}} ORDER BY sim DESC LIMIT 3",
        )
        .await
    })
    .await
    .unwrap();

    assert_eq!(rows.len(), 3);
    let scores: Vec<f32> = rows
        .iter()
        .map(|row| {
            row.fields
                .get("sim")
                .and_then(Value::as_f32)
                .expect("sim score should be present")
        })
        .collect();
    // Scores should be returned in descending order.
    for window in scores.windows(2) {
        assert!(
            window[0] >= window[1],
            "scores not in descending order: {scores:?}"
        );
    }
}

#[rstest]
#[case::select_star("SELECT * FROM {{table}}", "Unsupported: SELECT *")]
#[case::missing_alias(
    "SELECT published_year + 1 FROM {{table}}",
    "Invalid: expression in SELECT list requires an AS alias"
)]
#[case::missing_from("SELECT _id LIMIT 1", "Invalid: SELECT requires a FROM clause")]
#[case::missing_table("SELECT _id FROM never_made_this LIMIT 1", "Table does not exist")]
#[case::order_limit(
    "SELECT _id FROM {{table}} ORDER BY published_year",
    "Invalid: ORDER BY without LIMIT is not supported"
)]
#[case::string_sort(
    "SELECT _id FROM {{table}} ORDER BY author ASC LIMIT 3",
    "Invalid argument: Input to SortWithLimit must produce primitive type, not String"
)]
#[case::ordinal_sort(
    "SELECT _id FROM {{table}} ORDER BY 1 LIMIT 5",
    "Unsupported: ORDER BY with ordinal position is not supported"
)]
#[case::zero_limit(
    "SELECT _id FROM {{table}} LIMIT 0",
    "Invalid argument: Invalid argument: Limit k must be > 0"
)]
#[case::negative_limit(
    "SELECT _id FROM {{table}} LIMIT -1",
    "Invalid: LIMIT must be a positive integer"
)]
#[case::ilike(
    "SELECT _id FROM {{table}} WHERE _id ILIKE 'h%' LIMIT 5",
    "Unsupported: ILIKE: TopK has no case-insensitive matching primitive"
)]
#[case::cast_filter(
    "SELECT _id FROM {{table}} WHERE CAST(rating AS INTEGER) > 4 LIMIT 5",
    "Unsupported: explicit CAST is only supported in SELECT"
)]
#[case::distinct_from(
    "SELECT _id FROM {{table}} WHERE genre IS DISTINCT FROM 'fiction' LIMIT 5",
    "Unsupported: IS DISTINCT FROM: not supported"
)]
#[case::in_subquery(
    "SELECT _id FROM {{table}} WHERE _id IN (SELECT _id FROM {{table}}) LIMIT 5",
    "Unsupported: IN (SELECT …): not supported"
)]
#[case::in_unnest(
    "SELECT _id FROM {{table}} WHERE published_year IN UNNEST(ARRAY[1937, 1949]) LIMIT 5",
    "Unsupported: IN UNNEST(…): not supported"
)]
#[case::group_by(
    "SELECT _id FROM {{table}} GROUP BY genre LIMIT 5",
    "Unsupported: GROUP BY"
)]
#[case::distinct(
    "SELECT DISTINCT genre FROM {{table}} LIMIT 5",
    "Unsupported: SELECT DISTINCT"
)]
#[case::join(
    "SELECT _id FROM {{table}} a JOIN {{table}} b ON a._id = b._id LIMIT 5",
    "Unsupported: JOIN"
)]
#[case::offset("SELECT _id FROM {{table}} LIMIT 5 OFFSET 3", "Unsupported: OFFSET")]
#[case::unknown_fn(
    "SELECT _id FROM {{table}} WHERE FOOBAR(rating) > 0",
    "Unknown function: FOOBAR"
)]
#[case::count_distinct(
    "SELECT COUNT(DISTINCT genre) FROM {{table}}",
    "Unsupported: only COUNT(*) is supported; COUNT(expr) and DISTINCT are not"
)]
#[case::count_expr(
    "SELECT COUNT(author) FROM {{table}}",
    "Unsupported: only COUNT(*) is supported; COUNT(expr) and DISTINCT are not"
)]
#[case::count_with_other_columns(
    "SELECT COUNT(*), title FROM {{table}}",
    "Unsupported: COUNT(*) cannot be combined with other columns"
)]
#[case::like_wildcard(
    "SELECT _id FROM {{table}} WHERE title LIKE 'The%Rings%'",
    "Unsupported: LIKE pattern `The%Rings%` contains an unsupported wildcard"
)]
#[case::with_cte(
    "WITH cte AS (SELECT _id FROM {{table}}) SELECT _id FROM cte LIMIT 5",
    "Unsupported: WITH (common table expressions)"
)]
#[case::multi_order_by(
    "SELECT _id FROM {{table}} ORDER BY published_year ASC, rating DESC LIMIT 5",
    "Unsupported: ORDER BY with multiple keys is not supported"
)]
#[case::nulls_first(
    "SELECT _id FROM {{table}} ORDER BY published_year NULLS FIRST LIMIT 5",
    "Unsupported: ORDER BY \u{2026} NULLS FIRST/LAST"
)]
#[case::count_order_by(
    "SELECT COUNT(*) FROM {{table}} ORDER BY published_year ASC",
    "Unsupported: SELECT COUNT(*) ... ORDER BY ..."
)]
#[case::count_limit(
    "SELECT COUNT(*) FROM {{table}} LIMIT 5",
    "Unsupported: SELECT COUNT(*) ... LIMIT ..."
)]
#[case::union(
    "SELECT _id FROM {{table}} UNION SELECT _id FROM {{table}} LIMIT 5",
    "Unsupported: SELECT ... UNION/INTERSECT/EXCEPT ..."
)]
#[case::parse_bad_select(
    "SELECT FROM {{table}} LIMIT 1",
    "Parse error: sql parser error: Expected an expression, found: FROM at Line: 1, Column: 13"
)]
#[case::abs_too_many_args(
    "SELECT abs(published_year, rating) AS r FROM {{table}} LIMIT 1",
    "Invalid: abs: expected 1 args, got 2"
)]
#[case::coalesce_too_few_args(
    "SELECT coalesce(rating) AS r FROM {{table}} LIMIT 1",
    "Invalid: coalesce: expected 2 args, got 1"
)]
#[case::contains_too_few_args(
    "SELECT _id FROM {{table}} WHERE contains(title)",
    "Invalid: contains: expected 2 args, got 1"
)]
#[case::vector_distance_too_few_args(
    "SELECT vector_distance(embedding) AS s FROM {{table}} ORDER BY s LIMIT 1",
    "Invalid: vector_distance: expected 2..=3 args, got 1"
)]
#[case::multi_vector_too_few_args(
    "SELECT multi_vector_distance(multi_emb) AS s FROM {{table}} ORDER BY s LIMIT 1",
    "Invalid: multi_vector_distance: expected 2..=3 args, got 1"
)]
#[case::semantic_sim_too_few_args(
    "SELECT semantic_similarity(bio) AS s FROM {{table}} ORDER BY s LIMIT 1",
    "Invalid: semantic_similarity: expected 2 args, got 1"
)]
#[case::boost_too_few_args(
    "SELECT boost(rating, in_print) AS r FROM {{table}} LIMIT 1",
    "Invalid: boost: expected 3 args, got 2"
)]
#[case::bm25_one_arg(
    "SELECT bm25_score(0.75) AS s FROM {{table}} ORDER BY s LIMIT 5",
    "Invalid: bm25_score: expected 0 or 2 args, got 1"
)]
#[case::bm25_three_args(
    "SELECT bm25_score(0.75, 1.2, 0.5) AS s FROM {{table}} ORDER BY s LIMIT 5",
    "Invalid: bm25_score: expected 0 or 2 args, got 3"
)]
#[case::vector_distance_in_where(
    "SELECT _id FROM {{table}} WHERE vector_distance(embedding, f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0])) > 0",
    "Unsupported: `vector_distance` is a search function \u{2014} only valid at the top of a SELECT projection item (e.g. `SELECT vector_distance(\u{2026}) AS s FROM c ORDER BY s LIMIT k`)"
)]
#[case::in_list_non_literal(
    "SELECT _id FROM {{table}} WHERE published_year IN (published_year - 0) LIMIT 5",
    "Unsupported: IN list must contain only literal values"
)]
#[case::like_suffix_only(
    "SELECT _id FROM {{table}} WHERE title LIKE '%rings'",
    "Unsupported: LIKE pattern `%rings` is not supported"
)]
#[case::like_underscore(
    "SELECT _id FROM {{table}} WHERE title LIKE 'f_o'",
    "Unsupported: LIKE pattern `f_o` is not supported"
)]
#[tokio::test]
async fn rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}
