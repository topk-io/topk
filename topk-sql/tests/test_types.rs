use rstest::rstest;
use topk_rs::{doc, proto::v1::data::Value};

mod common;
use common::{BooksContext, Scope, SqlClient};

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

#[rstest]
#[case::ordinary_query(false, false)]
#[case::batch_query(true, false)]
#[case::prepared_query(false, true)]
#[tokio::test]
#[ignore = "requires PostgreSQL; run test_types with --ignored and PG connection variables"]
async fn decoding_errors(#[case] batch: bool, #[case] prepared: bool) {
    let client = SqlClient::from_env().await.unwrap();
    let sql = "SELECT 1::bigint AS good, 1::numeric AS bad";
    let result = if batch {
        client.batch(&[sql]).await
    } else if prepared {
        client.prepared(sql, vec![]).await
    } else {
        client.query(sql).await
    };
    let error = format!("{:#}", result.unwrap_err());
    assert!(
        error.contains("decoding column \"bad\" (numeric)"),
        "{error}"
    );
    assert!(error.contains("mismatched types"), "{error}");
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run test_types with --ignored and PG connection variables"]
async fn decoded_values() {
    let client = SqlClient::from_env().await.unwrap();
    let sql = "SELECT 1::smallint AS small, 2::integer AS medium, 9223372036854775807::bigint AS big, 1.5::real AS single, 1.25::double precision AS double, true AS boolean, 'hello'::text AS text, decode('00ff', 'hex') AS bytes, NULL::bigint AS missing, 'null'::json AS json_null, '42'::jsonb AS json_number, '[1,2]'::json AS json_list";
    let expected = vec![doc!(
        "small" => 1_i64, "medium" => 2_i64, "big" => i64::MAX,
        "single" => 1.5_f32, "double" => 1.25_f64, "boolean" => true,
        "text" => "hello", "bytes" => Value::bytes(vec![0, 255]),
        "missing" => Value::null(), "json_null" => Value::null(),
        "json_number" => 42_i64, "json_list" => "[1,2]"
    )];
    assert_eq!(client.query(sql).await.unwrap(), expected);
    assert_eq!(client.batch(&[sql]).await.unwrap(), expected);
    assert_eq!(client.prepared(sql, vec![]).await.unwrap(), expected);
}
