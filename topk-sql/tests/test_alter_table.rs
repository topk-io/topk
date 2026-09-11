use rstest::rstest;
use topk_rs::{doc, proto::v1::data::Document};

mod common;
use common::{Scope, TableScope};

#[rstest]
#[case::add_column(
    &[
        "CREATE TABLE {{table}} (name TEXT NOT NULL)",
        "ALTER TABLE {{table}} ADD COLUMN pages INTEGER",
        "INSERT INTO {{table}} (_id, name, pages) VALUES ('a', 'Dune', 412)",
    ],
    "SELECT pages FROM {{table}} WHERE _id = 'a'",
    vec![doc!("pages" => 412_i64)],
)]
#[case::add_column_if_not_exists(
    &[
        "CREATE TABLE {{table}} (name TEXT NOT NULL)",
        "ALTER TABLE {{table}} ADD COLUMN IF NOT EXISTS name INTEGER",
        "INSERT INTO {{table}} (_id, name) VALUES ('a', 'Dune')",
    ],
    "SELECT name FROM {{table}} WHERE _id = 'a'",
    vec![doc!("name" => "Dune")],
)]
#[case::drop_not_null(
    &[
        "CREATE TABLE {{table}} (name TEXT NOT NULL, pages INTEGER)",
        "ALTER TABLE {{table}} ALTER COLUMN name DROP NOT NULL",
        "INSERT INTO {{table}} (_id, pages) VALUES ('a', 412)",
    ],
    "SELECT pages FROM {{table}} WHERE _id = 'a'",
    vec![doc!("pages" => 412_i64)],
)]
#[case::set_not_null(
    &[
        "CREATE TABLE {{table}} (name TEXT)",
        "INSERT INTO {{table}} (_id, name) VALUES ('a', 'Dune')",
        "ALTER TABLE {{table}} ALTER COLUMN name SET NOT NULL",
    ],
    "SELECT name FROM {{table}} WHERE _id = 'a'",
    vec![doc!("name" => "Dune")],
)]
#[case::drop_column(
    &[
        "CREATE TABLE {{table}} (name TEXT NOT NULL, pages INTEGER)",
        "INSERT INTO {{table}} (_id, name, pages) VALUES ('a', 'Dune', 412)",
        "ALTER TABLE {{table}} DROP COLUMN name",
        "INSERT INTO {{table}} (_id, pages) VALUES ('b', 320)",
    ],
    "SELECT pages FROM {{table}} WHERE name = 'Dune'",
    vec![doc!("pages" => 412_i64)],
)]
#[case::type_reasserted(
    &[
        "CREATE TABLE {{table}} (name TEXT NOT NULL, pages INTEGER)",
        "ALTER TABLE {{table}} ALTER COLUMN name TYPE TEXT",
        "INSERT INTO {{table}} (_id, name, pages) VALUES ('a', 'Dune', 412)",
    ],
    "SELECT pages FROM {{table}} WHERE _id = 'a'",
    vec![doc!("pages" => 412_i64)],
)]
#[case::several_actions(
    &[
        "CREATE TABLE {{table}} (name TEXT NOT NULL, pages INTEGER)",
        "ALTER TABLE {{table}} ADD COLUMN isbn TEXT, DROP COLUMN pages, ALTER COLUMN name DROP NOT NULL",
        "INSERT INTO {{table}} (_id, isbn) VALUES ('a', '9780441013593')",
    ],
    "SELECT isbn FROM {{table}} WHERE _id = 'a'",
    vec![doc!("isbn" => "9780441013593")],
)]
#[tokio::test]
async fn alter_table_round_trip(
    #[case] setup: &[&str],
    #[case] select_sql: &str,
    #[case] expected: Vec<Document>,
) {
    let rows = TableScope::with_scope(async |ctx| {
        for sql in setup {
            ctx.sql(sql).await?;
        }
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(rows, expected);
}

#[rstest]
#[case::rename_column(
    &["CREATE TABLE {{table}} (name TEXT NOT NULL)"],
    "ALTER TABLE {{table}} RENAME COLUMN name TO title",
    "Unsupported: ALTER TABLE … RENAME COLUMN",
)]
#[case::set_default(
    &["CREATE TABLE {{table}} (name TEXT NOT NULL)"],
    "ALTER TABLE {{table}} ALTER COLUMN name SET DEFAULT 'x'",
    "Unsupported: ALTER COLUMN … SET DEFAULT",
)]
#[case::unknown_column(
    &["CREATE TABLE {{table}} (name TEXT NOT NULL)"],
    "ALTER TABLE {{table}} ALTER COLUMN nope SET NOT NULL",
    "column `nope` does not exist",
)]
#[case::type_change(
    &["CREATE TABLE {{table}} (name TEXT NOT NULL)"],
    "ALTER TABLE {{table}} ALTER COLUMN name TYPE INTEGER",
    "type changes unsupported",
)]
#[case::not_null_conflicts_with_data(
    &[
        "CREATE TABLE {{table}} (name TEXT, pages INTEGER)",
        "INSERT INTO {{table}} (_id, pages) VALUES ('a', 412)",
    ],
    "ALTER TABLE {{table}} ALTER COLUMN name SET NOT NULL",
    "name",
)]
#[case::drop_unknown_column(
    &["CREATE TABLE {{table}} (name TEXT NOT NULL)"],
    "ALTER TABLE {{table}} DROP COLUMN nope",
    "unknown",
)]
#[tokio::test]
async fn alter_table_rejected(
    #[case] setup: &[&str],
    #[case] failing_sql: &str,
    #[case] expected: &str,
) {
    let err = TableScope::with_scope(async |ctx| -> anyhow::Result<()> {
        for sql in setup {
            ctx.sql(sql).await?;
        }
        ctx.sql(failing_sql).await?;
        Ok(())
    })
    .await
    .unwrap_err();

    assert!(
        err.to_string().contains(expected),
        "expected `{expected}` in `{err}`"
    );
}
