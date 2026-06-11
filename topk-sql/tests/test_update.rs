use std::collections::HashSet;

use rstest::rstest;
use topk_rs::doc;
use topk_rs::proto::v1::data::Document;

mod common;
use common::{BooksContext, Scope, ids};

#[rstest]
#[case::single_field(
    "UPDATE {{table}} SET rating = 5.0 WHERE _id = 'gatsby'",
    "SELECT rating FROM {{table}} WHERE _id = 'gatsby'",
    vec![doc!("rating" => 5.0_f64)],
)]
#[case::multi_field(
    "UPDATE {{table}} SET rating = 5.0, genre = 'classic' WHERE _id = 'pride'",
    "SELECT rating, genre FROM {{table}} WHERE _id = 'pride'",
    vec![doc!("rating" => 5.0_f64, "genre" => "classic")],
)]
#[case::text_field(
    "UPDATE {{table}} SET genre = 'reread' WHERE _id = 'gatsby'",
    "SELECT genre FROM {{table}} WHERE _id = 'gatsby'",
    vec![doc!("genre" => "reread")],
)]
#[tokio::test]
async fn update(
    #[case] update_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: Vec<Document>,
) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(update_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(rows, expected);
}

#[rstest]
#[case::multi_match(
    "UPDATE {{table}} SET in_print = false WHERE _id IN ('hobbit','lotr','harry')",
    "SELECT _id FROM {{table}} WHERE in_print = false",
    ids!["hobbit", "lotr", "harry", "catcher", "moby"],
)]
#[case::single_match(
    "UPDATE {{table}} SET in_print = false WHERE _id = 'pride'",
    "SELECT _id FROM {{table}} WHERE in_print = false",
    ids!["pride", "catcher", "moby"],
)]
#[tokio::test]
async fn update_then_filter(
    #[case] update_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: HashSet<&str>,
) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(update_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::null_field(
    "UPDATE {{table}} SET genre = NULL WHERE _id = 'gatsby'",
    "SELECT _id FROM {{table}} WHERE genre IS NULL",
    ids!["gatsby"],
)]
#[case::dense_vector(
    "UPDATE {{table}} SET embedding = f32_vector(ARRAY[0, 0, 0, 1]) WHERE _id = 'hobbit'",
    "SELECT _id FROM {{table}} WHERE embedding IS NOT NULL AND _id = 'hobbit'",
    ids!["hobbit"],
)]
#[case::dense_vector_cast(
    "UPDATE {{table}} SET embedding = '[0, 0, 0, 1]'::f32_vector WHERE _id = 'hobbit'",
    "SELECT _id FROM {{table}} WHERE embedding IS NOT NULL AND _id = 'hobbit'",
    ids!["hobbit"],
)]
#[tokio::test]
async fn update_optional_fields(
    #[case] update_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: HashSet<&str>,
) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql(update_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::missing_where(
    "UPDATE {{table}} SET rating = 0",
    "Invalid: UPDATE requires a `WHERE _id = …` or `WHERE _id IN (…)` clause"
)]
#[case::id_assignment(
    "UPDATE {{table}} SET _id = 'new' WHERE _id = 'gatsby'",
    "Invalid: cannot UPDATE the `_id` field"
)]
#[case::duplicate_field(
    "UPDATE {{table}} SET rating = 5.0, rating = 4.0 WHERE _id = 'pride'",
    "Invalid: field `rating` assigned more than once"
)]
#[case::filter_where(
    "UPDATE {{table}} SET rating = 5.0 WHERE genre = 'fiction'",
    "Invalid: UPDATE requires a `WHERE _id = \u{2026}` or `WHERE _id IN (\u{2026})` clause"
)]
#[case::update_from(
    "UPDATE {{table}} SET rating = 5.0 FROM {{table}} AS src WHERE {{table}}._id = src._id",
    "Unsupported: UPDATE \u{2026} FROM"
)]
#[case::update_returning(
    "UPDATE {{table}} SET rating = 5.0 WHERE _id = 'gatsby' RETURNING *",
    "Unsupported: UPDATE \u{2026} RETURNING"
)]
#[tokio::test]
async fn update_rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}
