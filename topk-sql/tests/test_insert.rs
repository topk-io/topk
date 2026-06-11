use std::collections::HashSet;

use rstest::rstest;
use topk_rs::doc;
use topk_rs::proto::v1::data::Document;

mod common;
use common::{BooksContext, Scope, TableScope, ids};

#[rstest]
#[case::new_doc(
    "INSERT INTO {{table}} (_id, title, author, published_year, rating, genre, in_print) VALUES ('extra', 'Extra Book', 'Anon', 2020, 4.0, 'fiction', true)",
    "SELECT title, author FROM {{table}} WHERE _id = 'extra'",
    vec![doc!("title" => "Extra Book", "author" => "Anon")],
)]
#[case::replace_doc(
    "INSERT INTO {{table}} (_id, title, author, published_year, rating, genre, in_print) VALUES ('hobbit', 'Hobbit Reissue', 'Tolkien', 2024, 5.0, 'fantasy', true)",
    "SELECT title FROM {{table}} WHERE _id = 'hobbit'",
    vec![doc!("title" => "Hobbit Reissue")],
)]
#[case::negative_int(
    "INSERT INTO {{table}} (_id, title, author, published_year, rating, genre, in_print) VALUES ('neg', 'Neg', 'A', -100, 4.0, 'fiction', true)",
    "SELECT published_year FROM {{table}} WHERE _id = 'neg'",
    vec![doc!("published_year" => -100_i64)],
)]
#[tokio::test]
async fn upsert(
    #[case] insert_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: Vec<Document>,
) {
    let rows = BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql(insert_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(rows, expected);
}

#[rstest]
#[case::omitted_float(
    "INSERT INTO {{table}} (_id, title) VALUES ('doc', 'Doc')",
    "SELECT _id FROM {{table}} WHERE _id = 'doc' AND rating IS NULL",
    ids!["doc"],
)]
#[tokio::test]
async fn upsert_omitted_null(
    #[case] insert_sql: &str,
    #[case] select_sql: &str,
    #[case] expected: HashSet<&str>,
) {
    let rows = BooksContext::with_scope(async |ctx: &BooksContext| {
        ctx.sql(insert_sql).await?;
        ctx.sql(select_sql).await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::dense_vector("embedding", "f32_vector(ARRAY[0.1, 0.2, 0.3, 0.4])", ids!["doc"])]
#[case::bytes("checksum", "bytes('deadbeef')", ids!["doc"])]
#[case::list("tags", "ARRAY['a', 'b', 'c']", ids!["doc"])]
#[case::sparse_vector(
    "sparse_emb",
    "f32_sparse_vector(ARRAY[0, 1], ARRAY[0.5, 0.5])",
    ids!["doc"],
)]
#[case::multi_vector(
    "multi_emb",
    "f32_matrix(ARRAY[ARRAY[0.1, 0.2, 0.3, 0.4]])",
    ids!["doc"],
)]
#[case::struct_value(
    "metadata",
    "struct('publisher', 'Anon Press', 'pages', 100)",
    ids!["doc"],
)]
#[case::f32_vector_cast("embedding", "'[0.1, 0.2, 0.3, 0.4]'::f32_vector", ids!["doc"])]
#[case::f32_sparse_cast(
    "sparse_emb",
    "'{\"0\": 0.5, \"1\": 0.5}'::f32_sparse_vector",
    ids!["doc"],
)]
#[case::f32_matrix_cast("multi_emb", "'[[0.1, 0.2, 0.3, 0.4]]'::f32_matrix", ids!["doc"])]
#[tokio::test]
async fn upsert_field_types(
    #[case] column: &str,
    #[case] value_sql: &str,
    #[case] expected: HashSet<&str>,
) {
    let rows = BooksContext::with_scope(async |ctx: &BooksContext| {
        let insert_sql = format!(
            "INSERT INTO {{{{table}}}} (_id, title, {column}) VALUES ('doc', 'Doc', {value_sql})"
        );

        ctx.sql(&insert_sql).await?;
        ctx.sql("SELECT _id FROM {{table}} WHERE _id = 'doc'").await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), expected);
}

#[rstest]
#[case::missing_id(
    "INSERT INTO {{table}} (title) VALUES ('Foo')",
    "Invalid: INSERT column list must include `_id`"
)]
#[case::duplicate_column(
    "INSERT INTO {{table}} (_id, title, title) VALUES ('x', 'A', 'B')",
    "Invalid: column `title` specified more than once"
)]
#[case::wrong_arity(
    "INSERT INTO {{table}} (_id, title) VALUES ('x', 'A', 'extra')",
    "Invalid: VALUES row has 3 entries, expected 2"
)]
#[case::select_source(
    "INSERT INTO {{table}} (_id, title) SELECT _id, title FROM {{table}}",
    "Unsupported: INSERT \u{2026} SELECT"
)]
#[case::no_column_list(
    "INSERT INTO {{table}} VALUES ('x', 'title')",
    "Invalid: INSERT requires an explicit column list"
)]
#[case::on_conflict(
    "INSERT INTO {{table}} (_id) VALUES ('x') ON CONFLICT DO NOTHING",
    "Unsupported: INSERT \u{2026} ON CONFLICT"
)]
#[case::invalid_bytes_hex(
    "INSERT INTO {{table}} (_id, checksum) VALUES ('x', bytes('xyz'))",
    "Invalid: bytes: hex string has odd length"
)]
#[case::hex_invalid_char(
    "INSERT INTO {{table}} (_id, checksum) VALUES ('x', bytes('zz'))",
    "Invalid: bytes: invalid hex `zz`: invalid digit found in string"
)]
#[case::odd_struct_args(
    "INSERT INTO {{table}} (_id, metadata) VALUES ('x', struct('a', 1, 'b'))",
    "Invalid: struct: expected (field, value) pairs"
)]
#[case::struct_int_key(
    "INSERT INTO {{table}} (_id, metadata) VALUES ('x', struct(1, 'a'))",
    "Invalid: struct: field name must be a string literal"
)]
#[case::integer_vector_range(
    "INSERT INTO {{table}} (_id, embedding) VALUES ('x', i8_vector(ARRAY[200]))",
    "Invalid: i8_vector: element 200 out of i8 range"
)]
#[case::cast_i8_vector_range(
    "INSERT INTO {{table}} (_id, embedding) VALUES ('x', '[200]'::i8_vector)",
    "Invalid: i8_vector: element 200 out of range"
)]
#[case::cast_u8_vector_range(
    "INSERT INTO {{table}} (_id, embedding) VALUES ('x', '[300]'::u8_vector)",
    "Invalid: u8_vector: element 300 out of range"
)]
#[case::sparse_len_mismatch(
    "INSERT INTO {{table}} (_id, sparse_emb) VALUES ('x', f32_sparse_vector(ARRAY[0, 1], ARRAY[0.5]))",
    "Invalid: f32_sparse_vector: index and value arrays must have equal length"
)]
#[case::matrix_row_mismatch(
    "INSERT INTO {{table}} (_id, multi_emb) VALUES ('x', f32_matrix(ARRAY[ARRAY[1, 2], ARRAY[3]]))",
    "Invalid: f32_matrix: all rows must have the same length (got 1 after 2)"
)]
#[case::matrix_no_rows(
    "INSERT INTO {{table}} (_id, multi_emb) VALUES ('x', f32_matrix(ARRAY[]))",
    "Invalid: f32_matrix: must have at least one row"
)]
#[case::empty_array(
    "INSERT INTO {{table}} (_id, tags) VALUES ('x', ARRAY[])",
    "Invalid: empty ARRAY[]: element type cannot be inferred"
)]
#[case::empty_dense_vector(
    "INSERT INTO {{table}} (_id, embedding) VALUES ('x', f32_vector(ARRAY[]))",
    "Invalid: f32_vector: must be non-empty"
)]
#[case::mixed_elem_types(
    "INSERT INTO {{table}} (_id, tags) VALUES ('x', ARRAY[1, 'a'])",
    "Invalid: ARRAY[...]: mixed element types"
)]
#[case::bool_elem_type(
    "INSERT INTO {{table}} (_id, tags) VALUES ('x', ARRAY[true])",
    "Invalid: ARRAY[...]: unsupported element type"
)]
#[tokio::test]
async fn upsert_rejected(#[case] query: &str, #[case] expected: &str) {
    let err = BooksContext::with_scope(async |ctx| ctx.sql(query).await)
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), expected);
}

#[rstest]
#[case::binary_vector(
    "CREATE TABLE {{table}} (vec binary_vector(4))",
    "INSERT INTO {{table}} (_id, vec) VALUES ('doc', binary_vector(ARRAY[1, 0, 1, 0]))",
    ids!["doc"],
)]
#[case::f16_matrix(
    "CREATE TABLE {{table}} (emb f16_matrix(4))",
    "INSERT INTO {{table}} (_id, emb) VALUES ('doc', f16_matrix(ARRAY[ARRAY[0.1, 0.2, 0.3, 0.4]]))",
    ids!["doc"],
)]
#[case::f8_matrix(
    "CREATE TABLE {{table}} (emb f8_matrix(4))",
    "INSERT INTO {{table}} (_id, emb) VALUES ('doc', f8_matrix(ARRAY[ARRAY[0.1, 0.2, 0.3, 0.4]]))",
    ids!["doc"],
)]
#[tokio::test]
async fn upsert_constructors(
    #[case] create_sql: &str,
    #[case] insert_sql: &str,
    #[case] expected: HashSet<&str>,
) {
    let rows = TableScope::with_scope(async |ctx| {
        ctx.sql(create_sql).await?;
        ctx.sql(insert_sql).await?;
        ctx.sql("SELECT _id FROM {{table}} WHERE _id = 'doc'").await
    })
    .await
    .unwrap();

    assert_eq!(ids(&rows), expected);
}
