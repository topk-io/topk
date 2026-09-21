use rstest::rstest;

mod common;
use common::{BooksContext, Scope};

#[tokio::test]
async fn read_at_write_lsn() {
    let rows = BooksContext::with_scope(async |ctx: &BooksContext| {
        let lsn = ctx
            .sql("INSERT INTO {{table}} (_id, title) VALUES ('pinned', 'Pinned') RETURNING _lsn")
            .await?;

        let lsn = lsn[0].fields["_lsn"].as_string().expect("_lsn is a string");

        ctx.sql(format!(
            "SELECT title FROM {{{{table}}}} WITH (required_lsn = {lsn}) WHERE _id = 'pinned'"
        ))
        .await
    })
    .await
    .unwrap();

    assert_eq!(rows.len(), 1, "the write must be visible at its own lsn");
}

#[tokio::test]
async fn unreachable_lsn_is_rejected() {
    let err = BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SELECT title FROM {{table}} WITH (required_lsn = 999999999999)")
            .await
    })
    .await
    .expect_err("an lsn that will never be reached must not be ignored");

    assert!(
        err.to_string().contains("timeout"),
        "expected the read to time out rather than ignore the lsn, got {err}"
    );
}

#[tokio::test]
async fn read_at_consistency() {
    let rows = BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SELECT title FROM {{table}} WITH (consistency = 'strong') LIMIT 1")
            .await
    })
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
}

#[tokio::test]
async fn set_consistency_level_is_gone() {
    BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SET consistency_level = 'strong'").await
    })
    .await
    .expect_err("consistency is a per-read option, not a session variable");
}

#[rstest]
#[case::unknown_option(
    "SELECT title FROM {{table}} WITH (nonsense = 1)",
    "Invalid: unknown option: nonsense"
)]
#[case::non_integer_lsn(
    "SELECT title FROM {{table}} WITH (required_lsn = 1.5)",
    "Invalid: required_lsn must be an integer, got 1.5"
)]
#[case::unknown_consistency(
    "SELECT title FROM {{table}} WITH (consistency = 'nonsense')",
    "Invalid: consistency must be 'indexed' or 'strong', got 'nonsense'"
)]
#[case::returning_not_lsn(
    "INSERT INTO {{table}} (_id, title) VALUES ('r', 'R') RETURNING title",
    "Unsupported: RETURNING other than `RETURNING _lsn`"
)]
#[case::returning_lsn_alias(
    "INSERT INTO {{table}} (_id, title) VALUES ('r', 'R') RETURNING 123 AS _lsn",
    "Unsupported: RETURNING other than `RETURNING _lsn`"
)]
#[case::returning_on_partition_delete(
    "DELETE FROM {{table}}$p1 RETURNING _lsn",
    "Unsupported: `RETURNING _lsn` on a partition delete"
)]
#[tokio::test]
async fn rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx: &BooksContext| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}
