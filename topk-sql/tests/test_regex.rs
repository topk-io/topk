use std::collections::HashSet;

use rstest::rstest;

mod common;
use common::{BooksContext, Scope, TableScope, ids};

#[rstest]
#[case::prefix("SELECT _id FROM {{table}} WHERE _id ~ '^h'", ids!["hobbit", "harry"])]
#[case::case_insensitive("SELECT _id FROM {{table}} WHERE author ~ '(?i)tolkien'", ids!["hobbit", "lotr"])]
#[case::suffix("SELECT _id FROM {{table}} WHERE _id ~ 'r$'", ids!["nineteen_eighty_four", "catcher", "lotr"])]
#[case::alternation("SELECT _id FROM {{table}} WHERE _id ~ '^(harry|hobbit)$'", ids!["harry", "hobbit"])]
#[case::char_class("SELECT _id FROM {{table}} WHERE _id ~ '^[a-c]'", ids!["alchemist", "catcher"])]
#[case::imatch("SELECT _id FROM {{table}} WHERE author ~* 'tolkien'", ids!["hobbit", "lotr"])]
#[case::not_match(
    "SELECT _id FROM {{table}} WHERE author !~ '^T'",
    ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "catcher", "harry", "alchemist", "moby"],
)]
#[case::regexp_like("SELECT _id FROM {{table}} WHERE regexp_like(_id, '^h')", ids!["hobbit", "harry"])]
#[case::regexp_like_flags(
    "SELECT _id FROM {{table}} WHERE regexp_like(author, 'tolkien', 'i')",
    ids!["hobbit", "lotr"],
)]
#[tokio::test]
async fn where_regex(#[case] query: &str, #[case] expected: HashSet<&str>) {
    let rows = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::backreference("SELECT _id FROM {{table}} WHERE author ~ '(a)\\1'")]
#[case::global_flag("SELECT _id FROM {{table}} WHERE regexp_like(author, 'a', 'g')")]
#[tokio::test]
async fn where_regex_invalid(#[case] query: &str) {
    BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .expect_err("query should fail");
}

#[rstest]
#[case::dot_matches_newline("WHERE note ~ 'line1.line2'", ids!["multiline"])]
#[case::newline_sensitive("WHERE regexp_like(note, '^line2$', 'n')", ids!["multiline"])]
#[case::not_match_skips_null("WHERE note !~ 'line1'", ids!["plain"])]
#[tokio::test]
async fn where_regex_newlines_and_nulls(#[case] filter: &str, #[case] expected: HashSet<&str>) {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql("CREATE TABLE {{table}} (note TEXT)").await?;
        ctx.sql(
            "INSERT INTO {{table}} (_id, note) VALUES \
             ('multiline', 'line1\nline2'), ('missing', NULL), ('plain', 'other')",
        )
        .await?;
        ctx.sql(&format!("SELECT _id FROM {{{{table}}}} {filter}"))
            .await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), expected);
}
