use rstest::rstest;
use topk_rs::{doc, proto::v1::data::Document};

mod common;
use common::{Scope, TableScope};

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
    "CREATE TABLE {{table}} (title TEXT NOT NULL, embedding f32_vector(4));
     CREATE INDEX ON {{table}} USING keyword_index (title);
     CREATE INDEX ON {{table}} USING vector_index (embedding) WITH (metric = 'cosine')",
    "INSERT INTO {{table}} (_id, title, embedding) VALUES ('doc', 'Hello World', f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0]))",
    "SELECT _id, vector_distance(embedding, f32_vector(ARRAY[1.0, 0.0, 0.0, 0.0])) AS d FROM {{table}} ORDER BY d LIMIT 1",
    vec![doc!("_id" => "doc", "d" => 1.0_f32)],
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
#[case::vector_index_unknown_option(
    "CREATE TABLE {{table}} (embedding f32_vector(4))",
    "CREATE INDEX ON {{table}} USING vector_index (embedding) WITH (metric = 'cosine', typo = 'oops')",
    "Invalid: unknown option `typo`"
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
