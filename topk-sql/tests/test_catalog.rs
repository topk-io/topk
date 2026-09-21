use rstest::rstest;
use topk_rs::proto::v1::data::{value, Document, Value};

mod common;
use common::{BooksContext, Scope, SessionContext};
use uuid::Uuid;

#[rstest]
#[case::information_schema_tables(
    "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    "table_name"
)]
#[case::pg_catalog_pg_tables(
    "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname = 'public'",
    "tablename"
)]
#[case::bare_pg_class("SELECT relname FROM pg_class WHERE relkind = 'r'", "relname")]
#[case::bare_pg_statio_user_tables(
    "SELECT relname FROM pg_statio_user_tables WHERE schemaname = 'public'",
    "relname"
)]
#[tokio::test]
async fn table_is_listed(#[case] sql: &str, #[case] field: &str) {
    BooksContext::with_scope(async |ctx| {
        let rows = ctx.sql(sql).await.unwrap();
        assert!(
            strings(&rows, field).contains(&ctx.table().to_string()),
            "created table should appear in {field} column"
        );
    })
    .await;
}

#[tokio::test]
async fn information_schema_columns() {
    BooksContext::with_scope(async |ctx| {
        let sql = format!(
            "SELECT column_name FROM information_schema.columns WHERE table_name = '{}'",
            ctx.table()
        );
        let rows = ctx.sql(&sql).await.unwrap();
        let columns = strings(&rows, "column_name");

        assert!(columns.contains(&"title".to_string()));
        assert!(columns.contains(&"author".to_string()));
        assert!(columns.contains(&"_id".to_string()));
    })
    .await;
}

#[rstest]
#[case::pg_type("SELECT typname FROM pg_catalog.pg_type LIMIT 10")]
#[case::pg_namespace("SELECT nspname FROM pg_catalog.pg_namespace")]
#[case::bare_pg_namespace("SELECT nspname FROM pg_namespace")]
#[case::bare_join(
    "SELECT n.nspname, c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace"
)]
#[tokio::test]
async fn returns_rows(#[case] sql: &str) {
    BooksContext::with_scope(async |ctx| {
        let rows = ctx.sql(sql).await.unwrap();
        assert!(!rows.is_empty());
    })
    .await;
}

#[rstest]
#[case::literal("SELECT 1", Value::string("1"))]
#[case::arithmetic("SELECT 1 + 1", Value::string("2"))]
#[case::version("SELECT version()", Value::string("PostgreSQL 16.0 (TopK)"))]
#[tokio::test]
async fn answers_without_a_table(#[case] sql: &str, #[case] expected: Value) {
    BooksContext::with_scope(async |ctx| {
        let rows = ctx.sql(sql).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fields.len(), 1);
        assert_eq!(rows[0].fields.values().next(), Some(&expected));
    })
    .await;
}

#[tokio::test]
async fn a_collection_named_like_the_catalog_is_read_through_quotes() {
    SessionContext::with_scope(async |ctx| {
        let table = format!("pg_probe_{}", Uuid::new_v4().simple());

        ctx.sql(&format!("CREATE TABLE {table} (title TEXT)"))
            .await
            .unwrap();
        ctx.sql(&format!(
            "INSERT INTO {table} (_id, title) VALUES ('a', 'Dune')"
        ))
        .await
        .unwrap();

        let rows = ctx
            .sql(&format!("SELECT title FROM \"{table}\" WHERE _id = 'a'"))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);

        // unquoted, the reserved prefix belongs to the catalog
        let err = ctx
            .sql(&format!("SELECT title FROM {table} WHERE _id = 'a'"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no such table"), "{err}");

        ctx.sql(&format!("DROP TABLE {table}")).await.unwrap();
    })
    .await;
}

fn strings(docs: &[Document], field: &str) -> Vec<String> {
    docs.iter()
        .filter_map(|doc| match doc.fields.get(field)?.value.as_ref()? {
            value::Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}
