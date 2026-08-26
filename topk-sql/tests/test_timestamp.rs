use std::collections::HashSet;

use rstest::rstest;

use topk_rs::doc;

mod common;
use common::{BooksContext, Scope, assert_rows_eq_unordered, ids};

// published_ts per book in the `books` fixture:
//   mockingbird 1960-07-11, nineteen_eighty_four 1949-06-08, pride 1813-01-28,
//   gatsby 1925-04-10, catcher 1951-07-16, moby 1851-10-18, hobbit 1937-09-21,
//   harry 1997-06-26, lotr 1954-07-29, alchemist 1988-01-01

#[rstest]
#[case::timestamp_literal(
    "SELECT _id FROM {{table}} WHERE published_ts < TIMESTAMP '1929-01-01'",
    ids!["pride", "moby", "gatsby"],
)]
#[case::timestamp_literal_utc(
    "SELECT _id FROM {{table}} WHERE published_ts < TIMESTAMP '1929-01-01T00:00:00Z'",
    ids!["pride", "moby", "gatsby"],
)]
#[case::timestamp_literal_with_offset(
    "SELECT _id FROM {{table}} WHERE published_ts < TIMESTAMP '1929-01-01T02:00:00+02:00'",
    ids!["pride", "moby", "gatsby"],
)]
#[case::extract_dow(
    "SELECT _id FROM {{table}} WHERE EXTRACT(DOW FROM published_ts) = 0",
    ids!["mockingbird", "catcher"],
)]
#[case::elapsed(
    "SELECT _id FROM {{table}} WHERE elapsed(published_ts, TIMESTAMP '1988-01-08', 'week') = 1",
    ids!["alchemist"],
)]
#[case::date_part_lt(
    "SELECT _id FROM {{table}} WHERE date_part('month', published_ts) < 6",
    ids!["gatsby", "pride", "alchemist"],
)]
#[case::extract_lt(
    "SELECT _id FROM {{table}} WHERE EXTRACT(MONTH FROM published_ts) < 6",
    ids!["gatsby", "pride", "alchemist"],
)]
#[tokio::test]
async fn timestamp_filter(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::tz_less_datetime(
    "SELECT _id FROM books WHERE published_ts < TIMESTAMP '1929-01-01 00:00:00'",
    "expected RFC 3339 with timezone offset"
)]
#[case::tz_less_datetime_isoformat(
    "SELECT _id FROM books WHERE published_ts < TIMESTAMP '1929-01-01T00:00:00'",
    "expected RFC 3339 with timezone offset"
)]
#[case::timestamptz_literal(
    "SELECT _id FROM books WHERE published_ts < TIMESTAMPTZ '1929-01-01T00:00:00Z'",
    "TIMESTAMPTZ"
)]
#[case::timestamptz_column("CREATE TABLE books (published_ts TIMESTAMPTZ)", "TIMESTAMPTZ")]
fn rejected_before_execution(#[case] sql: &str, #[case] expected_error: &str) {
    let err = topk_sql::convert_sql(topk_sql::parse_sql(sql).unwrap()).unwrap_err();
    assert!(
        err.to_string().contains(expected_error),
        "expected error containing `{expected_error}`, got: {err}"
    );
}

#[rstest]
#[case::date_part("date_part('year', published_ts)")]
#[case::extract("EXTRACT(YEAR FROM published_ts)")]
#[tokio::test]
async fn date_part_eq_field(#[case] year_expr: &str) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(&format!(
            "SELECT COUNT(*) FROM {{{{table}}}} WHERE {year_expr} = published_year"
        ))
        .await
    })
    .await
    .unwrap();

    assert_eq!(rows, vec![doc!("_count" => 10_i64)]);
}

#[tokio::test]
async fn date_part_group_by() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(
            "SELECT date_part('month', published_ts) AS published_month, COUNT(*) AS count \
             FROM {{table}} GROUP BY published_month",
        )
        .await
    })
    .await
    .unwrap();

    assert_rows_eq_unordered(
        rows,
        vec![
            doc!("published_month" => 1_i64, "count" => 2_i64),
            doc!("published_month" => 4_i64, "count" => 1_i64),
            doc!("published_month" => 6_i64, "count" => 2_i64),
            doc!("published_month" => 7_i64, "count" => 3_i64),
            doc!("published_month" => 9_i64, "count" => 1_i64),
            doc!("published_month" => 10_i64, "count" => 1_i64),
        ],
    );
}
