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

    println!("{err:?}");
}

#[tokio::test]
async fn unknown_option_is_rejected() {
    let err = BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SELECT title FROM {{table}} WITH (nonsense = 1)")
            .await
    })
    .await
    .expect_err("unknown options must not be silently ignored");

    println!("{err:?}");
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
async fn unknown_consistency_is_rejected() {
    BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SELECT title FROM {{table}} WITH (consistency = 'nonsense')")
            .await
    })
    .await
    .expect_err("consistency must be 'indexed' or 'strong'");
}

#[tokio::test]
async fn non_integer_required_lsn_is_rejected() {
    BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SELECT title FROM {{table}} WITH (required_lsn = 1.5)")
            .await
    })
    .await
    .expect_err("required_lsn must be an integer");
}

#[tokio::test]
async fn set_consistency_level_is_gone() {
    BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("SET consistency_level = 'strong'").await
    })
    .await
    .expect_err("consistency is a per-read option, not a session variable");
}

#[tokio::test]
async fn returning_other_than_lsn_is_rejected() {
    BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql("INSERT INTO {{table}} (_id, title) VALUES ('r', 'R') RETURNING title")
            .await
    })
    .await
    .expect_err("only RETURNING _lsn is supported");
}
