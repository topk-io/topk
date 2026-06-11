use rstest::rstest;
use topk_rs::proto::v1::data::Value;

mod common;
use common::{BooksContext, Scope};

#[rstest]
#[case::text("SELECT title FROM {{table}} LIMIT 1", "title", "text")]
#[case::integer(
    "SELECT published_year FROM {{table}} LIMIT 1",
    "published_year",
    "int8"
)]
#[case::float("SELECT rating FROM {{table}} LIMIT 1", "rating", "float8")]
#[case::boolean("SELECT in_print FROM {{table}} LIMIT 1", "in_print", "bool")]
#[case::bytea(
    "SELECT checksum FROM {{table}} WHERE checksum IS NOT NULL LIMIT 1",
    "checksum",
    "bytea"
)]
#[tokio::test]
async fn scalar_field_projection_type(
    #[case] query: &str,
    #[case] column: &str,
    #[case] expected_type: &str,
) {
    let col_type = BooksContext::with_scope(async |ctx| ctx.column_type(query, column).await)
        .await
        .unwrap();
    assert_eq!(col_type, expected_type, "column `{column}` type mismatch");
}

#[rstest]
#[case::list("SELECT tags FROM {{table}} WHERE _id = 'hobbit' LIMIT 1", "tags")]
#[case::struct_field(
    "SELECT metadata FROM {{table}} WHERE _id = 'mockingbird' LIMIT 1",
    "metadata"
)]
#[tokio::test]
async fn non_scalar_field_projection_type_is_json(#[case] query: &str, #[case] column: &str) {
    let col_type = BooksContext::with_scope(async |ctx| ctx.column_type(query, column).await)
        .await
        .unwrap();
    assert_eq!(col_type, "json", "column `{column}` should be json");
}

#[rstest]
#[case::int_to_text("SELECT published_year::text AS y FROM {{table}} LIMIT 1", "y", "text")]
#[case::float_to_float4("SELECT rating::float4 AS r FROM {{table}} LIMIT 1", "r", "float4")]
#[case::bool_to_text("SELECT in_print::text AS p FROM {{table}} LIMIT 1", "p", "text")]
#[tokio::test]
async fn explicit_cast_overrides_schema_type(
    #[case] query: &str,
    #[case] column: &str,
    #[case] expected_type: &str,
) {
    let col_type = BooksContext::with_scope(async |ctx| ctx.column_type(query, column).await)
        .await
        .unwrap();
    assert_eq!(
        col_type, expected_type,
        "column `{column}` cast override failed"
    );
}

#[rstest]
#[case::text("SELECT title FROM {{table}} WHERE _id = $1", vec![Value::string("hobbit")], "title", "text")]
#[case::integer("SELECT published_year FROM {{table}} WHERE _id = $1", vec![Value::string("hobbit")], "published_year", "int8")]
#[case::float("SELECT rating FROM {{table}} WHERE _id = $1", vec![Value::string("hobbit")], "rating", "float8")]
#[case::boolean("SELECT in_print FROM {{table}} WHERE _id = $1", vec![Value::string("hobbit")], "in_print", "bool")]
#[case::bytea("SELECT checksum FROM {{table}} WHERE _id = $1", vec![Value::string("hobbit")], "checksum", "bytea")]
#[tokio::test]
async fn extended_query_column_types(
    #[case] query: &str,
    #[case] params: Vec<Value>,
    #[case] column: &str,
    #[case] expected_type: &str,
) {
    let col_type =
        BooksContext::with_scope(async |ctx| ctx.prepared_column_type(query, params, column).await)
            .await
            .unwrap();
    assert_eq!(col_type, expected_type, "column `{column}` type mismatch");
}
