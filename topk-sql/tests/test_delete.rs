use std::collections::HashSet;

use rstest::rstest;
use topk_sql::{convert_sql, parse_sql};

mod common;
use common::{ids, BooksContext, Scope};

#[rstest]
#[case::id_eq(
    "DELETE FROM {{table}} WHERE _id = 'moby'",
    ids!["alchemist", "catcher", "gatsby", "harry", "hobbit", "lotr", "mockingbird", "nineteen_eighty_four", "pride"],
)]
#[case::id_in(
    "DELETE FROM {{table}} WHERE _id IN ('hobbit','lotr','harry')",
    ids!["alchemist", "catcher", "gatsby", "mockingbird", "moby", "nineteen_eighty_four", "pride"],
)]
#[case::text_eq(
    "DELETE FROM {{table}} WHERE genre = 'fantasy'",
    ids!["alchemist", "catcher", "gatsby", "mockingbird", "moby", "nineteen_eighty_four", "pride"],
)]
#[case::compound(
    "DELETE FROM {{table}} WHERE rating < 4.0 AND in_print = true",
    ids!["catcher", "harry", "hobbit", "lotr", "mockingbird", "moby", "nineteen_eighty_four", "pride"],
)]
#[case::regex(
    "DELETE FROM {{table}} WHERE _id ~ '^h'",
    ids!["alchemist", "catcher", "gatsby", "lotr", "mockingbird", "moby", "nineteen_eighty_four", "pride"],
)]
#[case::like(
    "DELETE FROM {{table}} WHERE _id LIKE 'm%'",
    ids!["alchemist", "catcher", "gatsby", "harry", "hobbit", "lotr", "nineteen_eighty_four", "pride"],
)]
#[case::delete_all("DELETE FROM {{table}} WHERE published_year > 0", ids![])]
#[tokio::test]
async fn delete(#[case] delete_sql: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql(delete_sql).await?;
        ctx.sql("SELECT _id FROM {{table}}").await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::missing_where(
    "DELETE FROM {{table}}",
    "Invalid: DELETE without a WHERE clause is not allowed"
)]
#[case::delete_returning(
    "DELETE FROM {{table}} WHERE _id = 'moby' RETURNING *",
    "Unsupported: RETURNING other than `RETURNING _lsn`"
)]
#[tokio::test]
async fn delete_rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}

#[rstest]
#[case::limit(
    "DELETE FROM books WHERE rating > 0 LIMIT 1",
    "Unsupported: DELETE … LIMIT"
)]
#[case::ordered(
    "DELETE FROM books WHERE rating > 0 ORDER BY rating",
    "Unsupported: DELETE … ORDER BY"
)]
#[case::partition_limit("DELETE FROM books$tenant LIMIT 1", "Unsupported: DELETE … LIMIT")]
fn delete_ignored_clause(#[case] sql: &str, #[case] expected: &str) {
    let err = convert_sql(parse_sql(sql).unwrap()).unwrap_err();
    assert_eq!(err.to_string(), expected);
}

#[tokio::test]
async fn rejected_limit_preserves_rows() {
    BooksContext::with_scope(async |ctx: &BooksContext| {
        let before_rows = ctx.sql("SELECT _id FROM {{table}}").await.unwrap();
        let before = ids(&before_rows);
        let err = ctx
            .sql("DELETE FROM {{table}} WHERE published_year > 0 LIMIT 1")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Unsupported: DELETE … LIMIT"));
        let after = ctx.sql("SELECT _id FROM {{table}}").await.unwrap();
        assert_eq!(ids(&after), before);
    })
    .await;
}
