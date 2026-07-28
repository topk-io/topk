use std::collections::HashSet;

use rstest::rstest;
use topk_rs::{doc, proto::v1::data::Document};

mod common;
use common::{Scope, TableScope, ids};

#[rstest]
#[case::minimal(
    "CREATE TABLE {{table}} (name TEXT NOT NULL, score FLOAT)",
    "INSERT INTO {{table}} (_id, name, score) VALUES ('a', 'Alice', 9.5)",
    "SELECT name FROM {{table}} WHERE _id = 'a'",
    vec![doc!("name" => "Alice")],
)]
#[case::scalar_types(
    "CREATE TABLE {{table}} (
        label    TEXT     NOT NULL,
        count    INTEGER,
        score    FLOAT,
        active   BOOLEAN,
        payload  BYTEA,
        meta     JSONB
    )",
    "INSERT INTO {{table}} (_id, label, count, score, active, payload, meta)
     VALUES ('r', 'hello', 42, 3.14, true, bytes('deadbeef'), struct('k', 'v'))",
    "SELECT label FROM {{table}} WHERE _id = 'r'",
    vec![doc!("label" => "hello")],
)]
#[case::indexes(
    "CREATE TABLE {{table}} (
        title     TEXT NOT NULL  INDEX keyword_index(),
        embedding f32_vector(4)  INDEX vector_index(metric = 'cosine')
    )",
    "INSERT INTO {{table}} (_id, title, embedding) VALUES ('doc', 'Hello World', f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0]))",
    "SELECT _id, vector_distance(embedding, f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0])) AS d FROM {{table}} ORDER BY d LIMIT 1",
    vec![doc!("_id" => "doc", "d" => 1.0_f32)],
)]
#[case::keyword_index_text_type(
    "CREATE TABLE {{table}} (title TEXT NOT NULL INDEX keyword_index(type = 'text'))",
    "INSERT INTO {{table}} (_id, title) VALUES ('doc', 'Hello World')",
    "SELECT _id FROM {{table}} WHERE match('hello', title) LIMIT 10",
    vec![doc!("_id" => "doc")],
)]
#[tokio::test]
async fn create_table_round_trip(
    #[case] create_sql: &str,
    #[case] insert_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: Vec<Document>,
) {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql(create_sql).await?;
        ctx.sql(insert_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(rows, expected);
}

#[rstest]
#[case::not_null(
    "CREATE TABLE {{table}} (name TEXT NOT NULL, score FLOAT)",
    "INSERT INTO {{table}} (_id, score) VALUES ('x', 1.0)",
    "Invalid row: ValidationErrorBag([MissingField { doc_id: \"x\", field: \"name\" }])"
)]
#[case::duplicate_table(
    "CREATE TABLE {{table}} (name TEXT NOT NULL)",
    "CREATE TABLE {{table}} (name TEXT NOT NULL)",
    "collection already exists"
)]
#[tokio::test]
async fn create_table_rejected(
    #[case] setup_sql: &str,
    #[case] failing_sql: &str,
    #[case] expected: &str,
) {
    let err = TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        ctx.sql(setup_sql).await?;
        ctx.sql(failing_sql).await?;
        Ok(())
    })
    .await
    .unwrap_err();

    assert_eq!(err.to_string(), expected);
}

#[rstest]
#[case::unknown_option(
    "CREATE TABLE {{table}} (
        name TEXT NOT NULL,
        embedding f32_vector(4) INDEX vector_index(metric = 'cosine', typo = 'oops')
    )",
    "Invalid: unknown option `typo`"
)]
#[case::keyword_index_invalid_type(
    "CREATE TABLE {{table}} (title TEXT NOT NULL INDEX keyword_index(type = 'oops'))",
    "Invalid: option `type` is invalid: invalid argument: invalid keyword index type `oops`, expected: text | exact"
)]
#[case::keyword_index_unknown_option(
    "CREATE TABLE {{table}} (title TEXT NOT NULL INDEX keyword_index(typo = 'text'))",
    "Invalid: unknown option `typo`"
)]
#[tokio::test]
async fn create_table_with_index_rejected(#[case] sql: &str, #[case] expected: &str) {
    let err = TableScope::with_scope(async |ctx| ctx.sql(sql).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}

#[rstest]
#[case::full_value("New York City", ids!["nyc"])]
#[case::partial_token("York", ids![])]
#[case::lowercased("new york city", ids![])]
#[case::camel_case("CamelCase", ids!["camel"])]
#[case::camel_case_lowercased("camelcase", ids![])]
#[tokio::test]
async fn exact_keyword_index(#[case] token: &str, #[case] expected: HashSet<&str>) {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql("CREATE TABLE {{table}} (tag TEXT NOT NULL INDEX keyword_index(type = 'exact'))")
            .await?;
        ctx.sql(
            "INSERT INTO {{table}} (_id, tag) \
             VALUES ('nyc', 'New York City'), ('camel', 'CamelCase')",
        )
        .await?;
        ctx.sql(&format!(
            "SELECT _id FROM {{{{table}}}} WHERE match('{token}', tag) LIMIT 10"
        ))
        .await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::noop(
    "CREATE TABLE {{table}} (name TEXT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS {{table}} (name TEXT NOT NULL)",
    "INSERT INTO {{table}} (_id, name) VALUES ('a', 'Alice')",
    "SELECT name FROM {{table}} WHERE _id = 'a'",
    vec![doc!("name" => "Alice")],
)]
#[tokio::test]
async fn create_table_if_not_exists(
    #[case] create_sql: &str,
    #[case] recreate_sql: &str,
    #[case] insert_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: Vec<Document>,
) {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql(create_sql).await?;
        ctx.sql(recreate_sql).await?;
        ctx.sql(insert_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(rows, expected);
}
