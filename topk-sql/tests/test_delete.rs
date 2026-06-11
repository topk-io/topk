use std::collections::HashSet;

use rstest::rstest;

mod common;
use common::{BooksContext, Scope, ids};

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
    "Unsupported: DELETE \u{2026} RETURNING"
)]
#[tokio::test]
async fn delete_rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}
