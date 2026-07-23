use std::collections::HashSet;

use rstest::rstest;

use topk_rs::{
    doc,
    proto::v1::data::{Document, Value, stage},
};

mod common;
use common::{BooksContext, Scope, assert_rows_eq_unordered, ids};

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
#[case::match_text_unqualified("SELECT _id FROM {{table}} WHERE match('rings')", ids!["lotr"])]
#[case::match_text("SELECT _id FROM {{table}} WHERE match('hobbit rings', title)", ids!["hobbit", "lotr"])]
#[case::match_text_and(
    "SELECT _id FROM {{table}} WHERE match('lord', title) AND match('rings', title)",
    ids!["lotr"],
)]
#[case::match_text_nested_or_with_logical(
    "SELECT _id FROM {{table}} WHERE (match('hobbit', title) OR match('rings', title)) AND published_year > 1940",
    ids!["lotr"],
)]
#[case::match_text_nested_or_and(
    "SELECT _id FROM {{table}} WHERE match('hobbit', title) OR (match('lord', title) AND match('rings', title))",
    ids!["hobbit", "lotr"],
)]
#[case::match_text_all(
    "SELECT _id FROM {{table}} WHERE match('kill mockingbird', title, 1.0, true)",
    ids!["mockingbird"],
)]
#[case::match_tokens(
    "SELECT _id FROM {{table}} WHERE match_tokens(ARRAY['hobbit', 'rings'], title)",
    ids!["hobbit", "lotr"],
)]
#[case::match_tokens_all(
    "SELECT _id FROM {{table}} WHERE match_tokens(ARRAY['lord', 'rings'], title, true)",
    ids!["lotr"],
)]
#[case::match_all("SELECT _id FROM {{table}} WHERE match_all(title, 'kill')", ids!["mockingbird"])]
#[case::match_all_tokens(
    "SELECT _id FROM {{table}} WHERE match_all(tags, ARRAY['epic', 'tolkien'])",
    ids!["lotr"],
)]
#[case::match_any(
    "SELECT _id FROM {{table}} WHERE match_any(title, 'hobbit rings')",
    ids!["hobbit", "lotr"],
)]
#[case::match_any_tokens(
    "SELECT _id FROM {{table}} WHERE match_any(tags, ARRAY['adventure', 'epic'])",
    ids!["hobbit", "lotr", "moby"],
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
#[case::bool_key(
    "SELECT (published_year < 1940) AS is_old, COUNT(*) AS count \
     FROM {{table}} GROUP BY is_old",
    vec![
        doc!("is_old" => true, "count" => 4_i64),
        doc!("is_old" => false, "count" => 6_i64),
    ],
)]
#[case::field_key(
    "SELECT genre, COUNT(*) AS count FROM {{table}} GROUP BY genre",
    vec![
        doc!("genre" => "fiction", "count" => 4_i64),
        doc!("genre" => "dystopian", "count" => 1_i64),
        doc!("genre" => "romance", "count" => 1_i64),
        doc!("genre" => "fantasy", "count" => 3_i64),
        doc!("genre" => "adventure", "count" => 1_i64),
    ],
)]
#[case::count_field_ignores_nulls(
    "SELECT (published_year < 1940) AS is_old, COUNT(*) AS total, \
     COUNT(nullable_importance) AS with_importance \
     FROM {{table}} GROUP BY is_old",
    vec![
        doc!("is_old" => true, "total" => 4_i64, "with_importance" => 1_i64),
        doc!("is_old" => false, "total" => 6_i64, "with_importance" => 1_i64),
    ],
)]
#[case::sum(
    "SELECT (published_year < 1940) AS is_old, SUM(published_year) AS total_year \
     FROM {{table}} GROUP BY is_old",
    // old: 1813 + 1925 + 1851 + 1937 = 7526; new: 1960 + 1949 + 1951 + 1997 + 1954 + 1988 = 11799
    vec![
        doc!("is_old" => true, "total_year" => 7526_i64),
        doc!("is_old" => false, "total_year" => 11799_i64),
    ],
)]
#[case::min_max(
    "SELECT (published_year < 1940) AS is_old, MIN(published_year) AS oldest, \
     MAX(published_year) AS newest \
     FROM {{table}} GROUP BY is_old",
    vec![
        doc!("is_old" => true, "oldest" => 1813_i64, "newest" => 1937_i64),
        doc!("is_old" => false, "oldest" => 1949_i64, "newest" => 1997_i64),
    ],
)]
#[case::avg(
    "SELECT (published_year < 1940) AS is_old, AVG(published_year) AS avg_year \
     FROM {{table}} GROUP BY is_old",
    vec![
        doc!("is_old" => true, "avg_year" => 1881.5_f64),
        doc!("is_old" => false, "avg_year" => 1966.5_f64),
    ],
)]
#[case::all_aggregations_combined(
    "SELECT (published_year < 1940) AS is_old, COUNT(*) AS count, \
     SUM(published_year) AS total_year, MIN(published_year) AS oldest, \
     MAX(published_year) AS newest, AVG(published_year) AS avg_year \
     FROM {{table}} GROUP BY is_old",
    vec![
        doc!(
            "is_old" => true, "count" => 4_i64, "total_year" => 7526_i64,
            "oldest" => 1813_i64, "newest" => 1937_i64, "avg_year" => 1881.5_f64
        ),
        doc!(
            "is_old" => false, "count" => 6_i64, "total_year" => 11799_i64,
            "oldest" => 1949_i64, "newest" => 1997_i64, "avg_year" => 1966.5_f64
        ),
    ],
)]
#[case::multiple_keys(
    // is_old  = published_year < 1940
    // is_19th = published_year < 1900
    //   pride 1813:  (old, 19th)      moby  1851:  (old, 19th)
    //   gatsby 1925: (old, !19th)     hobbit 1937: (old, !19th)
    //   the other 6: (!old, !19th)
    "SELECT (published_year < 1940) AS is_old, (published_year < 1900) AS is_19th, \
     COUNT(*) AS count \
     FROM {{table}} GROUP BY is_old, is_19th",
    vec![
        doc!("is_old" => true, "is_19th" => true, "count" => 2_i64),
        doc!("is_old" => true, "is_19th" => false, "count" => 2_i64),
        doc!("is_old" => false, "is_19th" => false, "count" => 6_i64),
    ],
)]
#[case::with_where(
    "SELECT (published_year > 1980) AS recent, COUNT(*) AS count \
     FROM {{table}} WHERE published_year >= 1940 GROUP BY recent",
    vec![
        doc!("recent" => true, "count" => 2_i64),
        doc!("recent" => false, "count" => 4_i64),
    ],
)]
#[case::with_having(
    "SELECT (published_year < 1940) AS is_old, COUNT(*) AS count \
     FROM {{table}} GROUP BY is_old HAVING count > 4",
    vec![doc!("is_old" => false, "count" => 6_i64)],
)]
#[case::with_order_by_limit(
    "SELECT (published_year < 1940) AS is_old, COUNT(*) AS count \
     FROM {{table}} GROUP BY is_old ORDER BY count DESC LIMIT 1",
    vec![doc!("is_old" => false, "count" => 6_i64)],
)]
#[tokio::test]
async fn group_by(#[case] query: &str, #[case] expected: Vec<Document>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();

    assert_rows_eq_unordered(rows, expected);
}

#[test]
fn group_by_with_having_direct_aggregate_call_rejected() {
    let sql = "SELECT genre, SUM(rating) AS total FROM books GROUP BY genre \
               HAVING SUM(rating) > 4";
    let err = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Unsupported: aggregate function calls in HAVING"
    );
}

#[test]
fn group_by_count_distinct_rejected_before_execution() {
    let sql = "SELECT genre, COUNT(DISTINCT author) AS author_count FROM books GROUP BY genre";
    let err = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Unsupported: COUNT: DISTINCT aggregates are not supported"
    );
}

#[test]
fn group_by_having_filter_clause_rejected_before_execution() {
    let sql = "SELECT genre, SUM(rating) AS s FROM books GROUP BY genre \
               HAVING SUM(rating) FILTER (WHERE in_print) > 4";
    let err = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Unsupported: aggregate function calls in HAVING"
    );
}

#[test]
fn group_by_column_alias_projects_alias_without_changing_group_key() {
    let sql = "SELECT genre AS g, COUNT(*) AS c FROM books GROUP BY genre";
    let mut converted = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap();
    assert_eq!(converted.len(), 1);

    let query = match converted.remove(0).0 {
        topk_sql::Statement::Select { query, .. } => query,
        other => panic!("expected SELECT statement, got {other:?}"),
    };

    assert_eq!(query.stages.len(), 2);

    let Some(stage::Stage::GroupBy(group)) = query.stages[0].stage.as_ref() else {
        panic!("expected GROUP BY stage");
    };
    assert!(group.keys.contains_key("genre"));
    assert!(group.keys.contains_key("g"));
    assert!(group.aggs.contains_key("c"));

    let Some(stage::Stage::Select(select)) = query.stages[1].stage.as_ref() else {
        panic!("expected final SELECT stage");
    };
    assert!(select.exprs.contains_key("g"));
    assert!(select.exprs.contains_key("c"));
    assert!(!select.exprs.contains_key("genre"));
}

#[test]
fn group_by_alias_shadowing_rejected_before_execution() {
    let sql = "SELECT author AS genre, COUNT(*) AS c FROM books GROUP BY genre";
    let err = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Unsupported: `genre` in a GROUP BY query must be a group key or an aggregate function call (COUNT/SUM/MIN/MAX/AVG)"
    );
}

#[test]
fn group_by_without_aggregate_rejected_before_execution() {
    let sql = "SELECT genre FROM books GROUP BY genre";
    let err = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Unsupported: GROUP BY queries require at least one aggregate function"
    );
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
#[case::string_sort(
    "SELECT _id FROM {{table}} ORDER BY author ASC LIMIT 3",
    vec!["pride", "alchemist", "gatsby"],
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
#[case::offset_only("SELECT _id FROM {{table}} LIMIT 4 OFFSET 3", 4)]
#[tokio::test]
async fn limit_offset(#[case] query: &str, #[case] expected_len: usize) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(rows.len(), expected_len);
}

#[tokio::test]
async fn sort_limit_offset() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT _id FROM {{table}} ORDER BY published_year ASC LIMIT 4 OFFSET 3")
            .await
    })
    .await
    .unwrap();
    let actual = rows.iter().map(|row| row.id().unwrap()).collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec!["hobbit", "nineteen_eighty_four", "catcher", "lotr"]
    );
}

#[rstest]
#[case::default_args(
    "SELECT _id, bm25_score() AS score FROM {{table}} WHERE match('rings', title) ORDER BY score DESC LIMIT 3",
    ids!["lotr"],
)]
#[case::custom_args(
    "SELECT _id, bm25_score(0.75, 1.2) AS score FROM {{table}} WHERE match('rings', title) ORDER BY score DESC LIMIT 3",
    ids!["lotr"],
)]
#[case::with_logical_filter(
    "SELECT _id, bm25_score() AS score FROM {{table}} WHERE match('hobbit rings', title) AND published_year > 1950 ORDER BY score DESC LIMIT 3",
    ids!["lotr"],
)]
#[case::match_tokens_filter(
    "SELECT _id, bm25_score() AS score FROM {{table}} WHERE match_tokens(ARRAY['hobbit', 'rings'], title) ORDER BY score DESC LIMIT 3",
    ids!["hobbit", "lotr"],
)]
#[tokio::test]
async fn bm25_search(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[tokio::test]
async fn bm25_match_weight_orders_results() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(
            "SELECT _id, bm25_score() AS score FROM {{table}} \
             WHERE match('rings', title, 10.0) OR match('hobbit', title, 1.0) \
             ORDER BY score DESC LIMIT 2",
        )
        .await
    })
    .await
    .unwrap();

    let actual = rows.iter().map(|row| row.id().unwrap()).collect::<Vec<_>>();
    assert_eq!(actual, vec!["lotr", "hobbit"]);
}

#[tokio::test]
async fn should_does_not_filter() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(
            "SELECT _id, bm25_score() AS score FROM {{table}} \
             WHERE should('rings', title) ORDER BY score DESC LIMIT 100",
        )
        .await
    })
    .await
    .unwrap();

    // should does not filter
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0].id().unwrap(), "lotr");
}

#[rstest]
#[case::boost_magic("magic", "harry")]
#[case::boost_epic("epic", "lotr")]
#[tokio::test]
async fn should_boosts_bm25_score(#[case] boost: &str, #[case] first: &str) {
    // the should term only affects ranking
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(&format!(
            "SELECT _id, bm25_score() AS score FROM {{{{table}}}} \
             WHERE match('fantasy', tags) AND should('{boost}', tags) \
             ORDER BY score DESC LIMIT 3"
        ))
        .await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), ids!["hobbit", "lotr", "harry"]);
    assert_eq!(rows[0].id().unwrap(), first);
}

#[tokio::test]
async fn text_filter_or_text_filter() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT _id FROM {{table}} WHERE match('hobbit', title) OR match('rings', title)")
            .await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), ids!["hobbit", "lotr"]);
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
#[case::group_by_non_key_column(
    "SELECT _id FROM {{table}} GROUP BY genre LIMIT 5",
    "Unsupported: `_id` in a GROUP BY query must be a group key or an aggregate function call (COUNT/SUM/MIN/MAX/AVG)"
)]
#[case::group_by_all(
    "SELECT genre, COUNT(*) AS c FROM {{table}} GROUP BY ALL",
    "Unsupported: GROUP BY ALL"
)]
#[case::group_by_rollup(
    "SELECT genre, COUNT(*) AS c FROM {{table}} GROUP BY ROLLUP(genre)",
    "Unsupported: GROUP BY key must be a column name or a SELECT-list alias"
)]
#[case::group_by_key_needs_alias(
    "SELECT COUNT(*) AS c FROM {{table}} GROUP BY published_year < 1940",
    "Unsupported: GROUP BY key must be a column name or a SELECT-list alias"
)]
#[case::having_without_group_by(
    "SELECT COUNT(*) AS c FROM {{table}} HAVING c > 1",
    "Invalid: HAVING requires a GROUP BY clause"
)]
#[case::distinct(
    "SELECT DISTINCT genre FROM {{table}} LIMIT 5",
    "Unsupported: SELECT DISTINCT"
)]
#[case::join(
    "SELECT _id FROM {{table}} a JOIN {{table}} b ON a._id = b._id LIMIT 5",
    "Unsupported: JOIN"
)]
#[case::offset_without_limit(
    "SELECT _id FROM {{table}} OFFSET 3",
    "Invalid: OFFSET without LIMIT is not supported"
)]
#[case::unknown_fn(
    "SELECT _id FROM {{table}} WHERE FOOBAR(rating) > 0",
    "Unknown function: FOOBAR"
)]
#[case::count_distinct(
    "SELECT COUNT(DISTINCT genre) FROM {{table}}",
    "Unsupported: non-COUNT(*) aggregate functions without GROUP BY"
)]
#[case::count_expr(
    "SELECT COUNT(author) FROM {{table}}",
    "Unsupported: non-COUNT(*) aggregate functions without GROUP BY"
)]
#[case::count_with_other_columns(
    "SELECT COUNT(*), title FROM {{table}}",
    "Unsupported: non-COUNT(*) aggregate functions without GROUP BY"
)]
#[case::aggregate_without_group_by(
    "SELECT SUM(rating) FROM {{table}}",
    "Unsupported: non-COUNT(*) aggregate functions without GROUP BY"
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
#[case::bm25_with_boolean_match(
    "SELECT _id, bm25_score() AS score FROM {{table}} WHERE match_any(title, 'rings') ORDER BY score DESC LIMIT 3",
    "Invalid argument: Invalid query: Query must have at least one text filter to compute bm25 scores"
)]
#[case::match_or_logical(
    "SELECT _id FROM {{table}} WHERE match('rings', title) OR published_year = 1813",
    "Unsupported: match/match_tokens can only be combined with logical filters using AND"
)]
#[case::match_tokens_or_logical(
    "SELECT _id FROM {{table}} WHERE match_tokens(ARRAY['rings'], title) OR published_year = 1813",
    "Unsupported: match/match_tokens can only be combined with logical filters using AND"
)]
#[case::match_non_string_query(
    "SELECT _id FROM {{table}} WHERE match(123, title)",
    "Invalid: match: query must be a string literal"
)]
#[case::match_non_bool_all(
    "SELECT _id FROM {{table}} WHERE match('rings', title, 1.0, 'yes')",
    "Invalid: match: all must be a bool literal"
)]
#[case::should_too_many_args(
    "SELECT _id FROM {{table}} WHERE should('rings', title, 1.0, true)",
    "Invalid: should: expected 1..=3 args, got 4"
)]
#[case::match_tokens_non_string_array(
    "SELECT _id FROM {{table}} WHERE match_tokens(ARRAY[1, 2], title)",
    "Invalid: match_tokens: tokens must be an array of strings"
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
