use rstest::rstest;
use topk_rs::doc;

mod common;
use common::{BooksContext, Scope};

#[tokio::test]
async fn tables_lists_created_table() {
    BooksContext::with_scope(async |ctx| {
        let rows = ctx
            .sql("SELECT table_name FROM information_schema.tables")
            .await
            .unwrap();

        assert!(
            rows.contains(&doc!("table_name" => ctx.table())),
            "expected {} in information_schema.tables, got: {rows:?}",
            ctx.table()
        );
    })
    .await;
}

#[rstest]
#[case::id("_id", "text")]
#[case::title("title", "text")]
#[case::author("author", "text")]
#[case::published_year("published_year", "integer")]
#[case::rating("rating", "float")]
#[case::genre("genre", "text")]
#[case::in_print("in_print", "boolean")]
#[case::bio("bio", "text")]
#[case::embedding("embedding", "f32_vector(4)")]
#[case::sparse_emb("sparse_emb", "f32_sparse_vector")]
#[case::multi_emb("multi_emb", "f32_matrix(4)")]
#[case::tags("tags", "text[]")]
#[case::checksum("checksum", "bytea")]
#[case::metadata("metadata", "jsonb")]
#[tokio::test]
async fn columns_schema(#[case] col: &str, #[case] dtype: &str) {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT column_name, data_type FROM information_schema.columns WHERE table_name = '{{table}}'")
            .await
    })
    .await
    .unwrap();

    assert!(
        rows.contains(&doc!("column_name" => col, "data_type" => dtype)),
        "expected ({col}, {dtype}), got: {rows:?}"
    );
}

#[tokio::test]
async fn columns_is_nullable_reflects_required() {
    let rows = BooksContext::with_scope(async |ctx| {
        ctx.sql("SELECT column_name, is_nullable FROM information_schema.columns WHERE table_name = '{{table}}'")
            .await
    })
    .await
    .unwrap();

    assert!(
        rows.contains(&doc!("column_name" => "_id", "is_nullable" => "NO")),
        "_id should be NOT NULL, got: {rows:?}"
    );
    assert!(
        rows.contains(&doc!("column_name" => "title", "is_nullable" => "NO")),
        "title (NOT NULL) should be NO, got: {rows:?}"
    );
    assert!(
        rows.contains(&doc!("column_name" => "rating", "is_nullable" => "YES")),
        "rating (nullable) should be YES, got: {rows:?}"
    );
}

#[rstest]
#[case::select_star(
    "SELECT * FROM information_schema.tables",
    "Invalid: SELECT * is not supported for information_schema; specify column names explicitly"
)]
#[case::columns_no_table_filter(
    "SELECT column_name FROM information_schema.columns WHERE table_schema = 'public'",
    "Invalid: information_schema.columns requires WHERE table_name = '<name>'"
)]
#[case::columns_no_where(
    "SELECT column_name FROM information_schema.columns",
    "Invalid: information_schema.columns requires WHERE table_name = '<name>'"
)]
#[case::columns_unknown_col(
    "SELECT attname FROM information_schema.columns WHERE table_name = 'x'",
    "column \"attname\" does not exist in information_schema.columns"
)]
#[case::tables_unknown_col(
    "SELECT nspname FROM information_schema.tables",
    "column \"nspname\" does not exist in information_schema.tables"
)]
#[tokio::test]
async fn rejected(#[case] sql: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(sql).await)
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), expected);
}
