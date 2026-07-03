use rstest::rstest;
use topk_rs::proto::v1::data::{Document, value};

mod common;
use common::{BooksContext, Scope};

#[rstest]
#[case::information_schema_tables(
    "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
    "table_name"
)]
#[case::pg_catalog_pg_tables(
    "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname = 'public'",
    "tablename"
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
#[tokio::test]
async fn returns_rows(#[case] sql: &str) {
    BooksContext::with_scope(async |ctx| {
        let rows = ctx.sql(sql).await.unwrap();
        assert!(!rows.is_empty());
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
