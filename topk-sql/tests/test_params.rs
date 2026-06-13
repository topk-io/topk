use std::collections::HashSet;

use rstest::rstest;
use topk_rs::doc;
use topk_rs::proto::v1::data::{Document, Value};

mod common;
use common::{BooksContext, Scope, ids};

#[rstest]
#[case::text("SELECT _id FROM {{table}} WHERE author = $1", vec![Value::string("Tolkien")], ids!["hobbit", "lotr"])]
#[case::int("SELECT _id FROM {{table}} WHERE published_year = $1", vec![Value::i32(1937)], ids!["hobbit"])]
#[case::float("SELECT _id FROM {{table}} WHERE rating >= $1", vec![Value::f32(4.5)], ids!["lotr", "harry"])]
#[case::bool_true("SELECT _id FROM {{table}} WHERE in_print = $1", vec![Value::bool(true)], ids!["mockingbird", "nineteen_eighty_four", "pride", "gatsby", "hobbit", "lotr", "harry", "alchemist"])]
#[case::bool_false("SELECT _id FROM {{table}} WHERE in_print = $1", vec![Value::bool(false)], ids!["catcher", "moby"])]
#[case::limit("SELECT _id FROM {{table}} ORDER BY published_year ASC LIMIT $1", vec![Value::i64(3)], ids!["pride", "moby", "gatsby"])]
#[case::multiple("SELECT _id FROM {{table}} WHERE published_year > $1 AND genre = $2", vec![Value::i32(1950), Value::string("fantasy")], ids!["lotr", "harry"])]
#[tokio::test]
async fn prepared_query(
    #[case] query: &str,
    #[case] params: Vec<Value>,
    #[case] expected: HashSet<&str>,
) {
    let rows = BooksContext::with_scope(async |ctx| ctx.prepared(query, params).await)
        .await
        .unwrap();
    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::text_eq(
    "DELETE FROM {{table}} WHERE genre = $1",
    vec![Value::string("fantasy")],
    ids!["alchemist", "catcher", "gatsby", "mockingbird", "moby", "nineteen_eighty_four", "pride"]
)]
#[case::int_eq(
    "DELETE FROM {{table}} WHERE published_year = $1",
    vec![Value::i32(1937)],
    ids!["alchemist", "catcher", "gatsby", "harry", "lotr", "mockingbird", "moby", "nineteen_eighty_four", "pride"]
)]
#[case::compound(
    "DELETE FROM {{table}} WHERE published_year > $1 AND genre = $2",
    vec![Value::i32(1950), Value::string("fantasy")],
    ids!["alchemist", "catcher", "gatsby", "hobbit", "mockingbird", "moby", "nineteen_eighty_four", "pride"]
)]
#[tokio::test]
async fn prepared_delete(
    #[case] delete_sql: &str,
    #[case] params: Vec<Value>,
    #[case] expected_remaining: HashSet<&str>,
) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.prepared(delete_sql, params).await?;
        ctx.sql("SELECT _id FROM {{table}}").await
    })
    .await
    .unwrap();
    assert_eq!(ids(&rows), expected_remaining);
}

#[rstest]
#[case::text_field(
    "UPDATE {{table}} SET genre = $1 WHERE _id = $2",
    vec![Value::string("reread"), Value::string("gatsby")],
    "SELECT genre FROM {{table}} WHERE _id = 'gatsby'",
    vec![doc!("genre" => "reread")],
)]
#[case::int_field(
    "UPDATE {{table}} SET published_year = $1 WHERE _id = $2",
    vec![Value::i32(1938), Value::string("hobbit")],
    "SELECT published_year FROM {{table}} WHERE _id = 'hobbit'",
    vec![doc!("published_year" => 1938_i64)],
)]
#[tokio::test]
async fn prepared_update(
    #[case] update_sql: &str,
    #[case] params: Vec<Value>,
    #[case] select_sql: &str,
    #[case] expected: Vec<Document>,
) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.prepared(update_sql, params).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();
    assert_eq!(rows, expected);
}
