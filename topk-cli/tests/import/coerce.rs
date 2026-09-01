use rstest::rstest;
use topk::import::{build_document, Error, Spec, Target};
use topk_rs::proto::v1::data::Value;

use crate::common::{json, refused, spec_toml};
use serde_json::json;

fn target(fields: &str) -> Target {
    crate::common::target("f.parquet", "_id", fields)
}

fn coerce(field: &str, value: Value) -> Result<Value, Error> {
    let target = target(&format!("v = {field}"));
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("v".to_string(), value),
    ];
    Ok(build_document(&target, record)?
        .fields
        .remove("v")
        .expect("field v"))
}

#[track_caller]
fn coerced(field: &str, value: Value) -> Value {
    coerce(field, value).expect("coerces")
}

/// One column may feed several fields — the id column included.
#[rstest]
fn fields_share_columns_and_the_id() {
    let target = target(
        "sku = { from = \"_id\", type = \"text\" }\n\
         a = { from = \"v\", type = \"int\" }\n\
         b = { from = \"v\", type = \"float\" }",
    );
    let doc = build_document(
        &target,
        vec![
            ("_id".to_string(), Value::string("1")),
            ("v".to_string(), Value::i64(7)),
        ],
    )
    .expect("document");
    assert_eq!(doc.fields.get("_id"), Some(&Value::string("1")));
    assert_eq!(doc.fields.get("sku"), Some(&Value::string("1")));
    assert_eq!(doc.fields.get("a"), Some(&Value::i64(7)));
    assert_eq!(doc.fields.get("b"), Some(&Value::f64(7.0)));
}

/// Every accepted (declared field, source cell) pair. A binary cell is a packed
/// little-endian array whose element width follows from `dim`, not the declared
/// type; a JSON string cell reshapes into any container.
#[rstest]
#[case::text_from_string(r#"{ type = "text" }"#, Value::string("hello"), Value::string("hello"))]
#[case::text_from_bool(r#"{ type = "text" }"#, Value::bool(true), Value::string("true"))]
#[case::text_from_int(r#"{ type = "text" }"#, Value::i64(42), Value::string("42"))]
#[case::text_from_float(r#"{ type = "text" }"#, Value::f64(4.3), Value::string("4.3"))]
#[case::text_from_utf8_bytes(r#"{ type = "text" }"#, Value::bytes(b"hello".to_vec()), Value::string("hello"))]
#[case::text_from_struct(r#"{ type = "text" }"#, Value::r#struct([("a", Value::i64(1))]), Value::string(r#"{"a":1}"#))]
#[case::truncate_cuts(
    r#"{ type = "text", truncate = 5 }"#,
    Value::string("hello world"),
    Value::string("hello")
)]
#[case::truncate_leaves_shorter(
    r#"{ type = "text", truncate = 5 }"#,
    Value::string("hi"),
    Value::string("hi")
)]
#[case::truncate_on_a_char_boundary(
    r#"{ type = "text", truncate = 5 }"#,
    Value::string("žluťoučký"),
    Value::string("žluťo")
)]
#[case::int_from_int(r#"{ type = "int" }"#, Value::i64(42), Value::i64(42))]
#[case::int_from_integral_float(r#"{ type = "int" }"#, Value::f64(4.0), Value::i64(4))]
#[case::int_from_bool(r#"{ type = "int" }"#, Value::bool(true), Value::i64(1))]
#[case::int_from_string(r#"{ type = "int" }"#, Value::string("42"), Value::i64(42))]
#[case::int_from_padded_string(r#"{ type = "int" }"#, Value::string("  42  "), Value::i64(42))]
#[case::int_from_negative_string(r#"{ type = "int" }"#, Value::string("-7"), Value::i64(-7))]
#[case::int_from_decimal_string(r#"{ type = "int" }"#, Value::string("3.00"), Value::i64(3))]
#[case::float_from_int(r#"{ type = "float" }"#, Value::i64(1), Value::f64(1.0))]
#[case::float_from_float(r#"{ type = "float" }"#, Value::f64(4.25), Value::f64(4.25))]
#[case::float_from_padded_string(
    r#"{ type = "float" }"#,
    Value::string(" 4.25 "),
    Value::f64(4.25)
)]
#[case::bool_from_bool(r#"{ type = "bool" }"#, Value::bool(true), Value::bool(true))]
#[case::bool_from_nonzero_int(r#"{ type = "bool" }"#, Value::i64(3), Value::bool(true))]
#[case::bool_from_zero_int(r#"{ type = "bool" }"#, Value::i64(0), Value::bool(false))]
#[case::bool_from_true_word(r#"{ type = "bool" }"#, Value::string("yes"), Value::bool(true))]
#[case::bool_from_false_word(r#"{ type = "bool" }"#, Value::string("no"), Value::bool(false))]
#[case::bool_from_false_digit(r#"{ type = "bool" }"#, Value::string("0"), Value::bool(false))]
#[case::bytes_stay_bytes(r#"{ type = "bytes" }"#, Value::bytes(vec![0xde, 0xad]), Value::bytes(vec![0xde, 0xad]))]
#[case::struct_from_json(r#"{ type = "struct" }"#, Value::string(r#"{"k": {"deep": 1}}"#), Value::r#struct([("k", Value::r#struct([("deep", Value::i64(1))]))]))]
#[case::int_list_from_ints(r#"{ type = "int_list" }"#, Value::list(vec![1_i64, 2]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_i32(r#"{ type = "int_list" }"#, Value::list(vec![1_i32, 2]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_integral_floats(r#"{ type = "int_list" }"#, Value::list(vec![1.0_f64, 2.0]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_json(r#"{ type = "int_list" }"#, Value::string("[1, 2]"), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_decimal_strings(r#"{ type = "int_list" }"#, Value::list(vec!["1.00", "2"]), Value::list(vec![1_i64, 2]))]
#[case::float_list_from_floats(r#"{ type = "float_list" }"#, Value::list(vec![0.5_f64, 1.5]), Value::list(vec![0.5_f32, 1.5]))]
#[case::float_list_from_ints(r#"{ type = "float_list" }"#, Value::list(vec![1_i64, 2]), Value::list(vec![1.0_f32, 2.0]))]
#[case::float_list_from_json(r#"{ type = "float_list" }"#, Value::string("[0.5, 1.5]"), Value::list(vec![0.5_f32, 1.5]))]
#[case::float_list_from_decimal_strings(r#"{ type = "float_list" }"#, Value::list(vec!["1.50", "2.25"]), Value::list(vec![1.5_f32, 2.25]))]
#[case::text_list_from_strings(r#"{ type = "text_list" }"#, Value::list(vec!["a", "b"]), Value::list(vec!["a", "b"]))]
#[case::text_list_from_json(r#"{ type = "text_list" }"#, Value::string(r#"["a", "b"]"#), Value::list(vec!["a", "b"]))]
#[case::vector_f32(r#"{ type = "f32_vector", dim = 3 }"#, Value::list(vec![1_i64, 2, 3]), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::vector_f32_from_pgvector_text(r#"{ type = "f32_vector", dim = 3 }"#, Value::string("[1,2,3]"), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::vector_f32_from_json_floats(r#"{ type = "f32_vector", dim = 3 }"#, Value::string("[0.5, 1.5, 2.5]"), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::vector_f32_from_floats(r#"{ type = "f32_vector", dim = 3 }"#, Value::list(vec![0.5_f64, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::vector_f16(r#"{ type = "f16_vector", dim = 3 }"#, Value::list(vec![1_i64, 2, 3]), Value::list(vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0)]))]
#[case::vector_f8(r#"{ type = "f8_vector", dim = 3 }"#, Value::list(vec![1_i64, 2, 3]), Value::list(vec![float8::F8E4M3::from_f32(1.0), float8::F8E4M3::from_f32(2.0), float8::F8E4M3::from_f32(3.0)]))]
#[case::vector_u8(r#"{ type = "u8_vector", dim = 3 }"#, Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::vector_u8_from_integral_floats(r#"{ type = "u8_vector", dim = 3 }"#, Value::list(vec![1.0_f64, 2.0, 3.0]), Value::list(vec![1_u8, 2, 3]))]
#[case::vector_i8(r#"{ type = "i8_vector", dim = 3 }"#, Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_i8, 2, 3]))]
#[case::vector_binary(r#"{ type = "binary_vector", dim = 3 }"#, Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::vector_f16_at_max(r#"{ type = "f16_vector", dim = 1 }"#, Value::list(vec![65504.0_f64]), Value::list(vec![half::f16::MAX]))]
#[case::vector_f8_in_range(r#"{ type = "f8_vector", dim = 1 }"#, Value::list(vec![400.0_f64]), Value::list(vec![float8::F8E4M3::from_f64(400.0)]))]
#[case::packed_f32_from_f16_bytes(r#"{ type = "f32_vector", dim = 3 }"#, le_f16(&[1.0, 2.0, 3.0]), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::packed_f32_from_f32_bytes(r#"{ type = "f32_vector", dim = 3 }"#, le_f32(&[0.5, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::packed_f32_from_f64_bytes(r#"{ type = "f32_vector", dim = 3 }"#, le_f64(&[0.5, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::packed_f16_from_f32_bytes(r#"{ type = "f16_vector", dim = 3 }"#, le_f32(&[1.0, 2.0, 3.0]), Value::list(vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0)]))]
#[case::packed_u8_from_bytes(r#"{ type = "u8_vector", dim = 3 }"#, Value::bytes(vec![1, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::packed_i8_from_signed_bytes(r#"{ type = "i8_vector", dim = 3 }"#, Value::bytes(vec![0xff, 2, 3]), Value::list(vec![-1_i8, 2, 3]))]
#[case::packed_binary_from_bytes(r#"{ type = "binary_vector", dim = 3 }"#, Value::bytes(vec![1, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::matrix_stays_a_matrix(r#"{ type = "f32_matrix", cols = 3 }"#, Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]))]
#[case::matrix_f32_from_a_flat_list(r#"{ type = "f32_matrix", cols = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]))]
#[case::matrix_f16_from_a_flat_list(r#"{ type = "f16_matrix", cols = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0), half::f16::from_f32(4.0), half::f16::from_f32(5.0), half::f16::from_f32(6.0)]))]
#[case::matrix_u8_from_a_flat_list(r#"{ type = "u8_matrix", cols = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1_u8, 2, 3, 4, 5, 6]))]
#[case::matrix_i8_from_a_flat_list(r#"{ type = "i8_matrix", cols = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1_i8, 2, 3, 4, 5, 6]))]
#[case::matrix_from_json_text(r#"{ type = "f32_matrix", cols = 2 }"#, Value::string("[1.0, 2.0, 3.0, 4.0]"), Value::matrix(2, vec![1.0_f32, 2.0, 3.0, 4.0]))]
#[case::sparse_f32_from_json(r#"{ type = "f32_sparse_vector" }"#, Value::string(r#"{"3": 1.5, "1": 0.5}"#), Value::sparse_vector(vec![1, 3], vec![0.5, 1.5]))]
#[case::sparse_f32_from_struct(r#"{ type = "f32_sparse_vector" }"#, Value::r#struct([("2", Value::f64(0.25))]), Value::sparse_vector(vec![2], vec![0.25]))]
#[case::sparse_u8_from_ints(r#"{ type = "u8_sparse_vector" }"#, Value::r#struct([("7", Value::i64(3))]), Value::sparse_vector(vec![7], vec![3u8]))]
#[case::sparse_skips_null_entries(r#"{ type = "f32_sparse_vector" }"#, Value::r#struct([("0", Value::f64(1.0)), ("3", Value::null()), ("7", Value::f64(0.5))]), Value::sparse_vector(vec![0, 7], vec![1.0, 0.5]))]
#[case::sparse_f32_from_parallel_lists(r#"{ type = "f32_sparse_vector" }"#, Value::r#struct([("indices", Value::list(vec![3_i64, 1])), ("values", Value::list(vec![1.5_f64, 0.5]))]), Value::sparse_vector(vec![1, 3], vec![0.5, 1.5]))]
#[case::sparse_u8_from_parallel_lists(r#"{ type = "u8_sparse_vector" }"#, Value::r#struct([("indices", Value::list(vec![7_i64])), ("values", Value::list(vec![3_i64]))]), Value::sparse_vector(vec![7], vec![3u8]))]
#[case::sparse_from_empty_parallel_lists(r#"{ type = "f32_sparse_vector" }"#, Value::r#struct([("indices", Value::list(Vec::<i64>::new())), ("values", Value::list(Vec::<f64>::new()))]), Value::sparse_vector(Vec::<u32>::new(), Vec::<f32>::new()))]
fn coerces(#[case] field: &str, #[case] input: Value, #[case] expected: Value) {
    assert_eq!(coerced(field, input), expected);
}

fn le_f16(ns: &[f32]) -> Value {
    Value::bytes(
        ns.iter()
            .flat_map(|n| half::f16::from_f32(*n).to_le_bytes())
            .collect::<Vec<u8>>(),
    )
}

fn le_f32(ns: &[f32]) -> Value {
    Value::bytes(ns.iter().flat_map(|n| n.to_le_bytes()).collect::<Vec<u8>>())
}

fn le_f64(ns: &[f64]) -> Value {
    Value::bytes(ns.iter().flat_map(|n| n.to_le_bytes()).collect::<Vec<u8>>())
}

/// A double beyond 2^53 has already dropped digits, so it cannot be an id —
/// a csv column mixing huge ids with decimals sniffs as double.
#[test]
fn imprecise_double_id_is_refused() {
    let target = target(r#"v = { type = "int" }"#);
    let record = vec![
        (
            "_id".to_string(),
            Value::f64(18446744073709551615_u64 as f64),
        ),
        ("v".to_string(), Value::i64(1)),
    ];
    let message = refused(build_document(&target, record));
    assert!(message.contains("lost integer precision"), "got: {message}");
}

#[test]
fn struct_invalid_json() {
    let message = refused(coerce(r#"{ type = "struct" }"#, Value::string("not json")));
    assert!(message.contains(r#"doc "1""#), "got: {message}");
    assert!(message.contains(r#"field "v""#), "got: {message}");
}

#[rstest]
fn nulls(
    #[values(
        r#"{ type = "text" }"#,
        r#"{ type = "f32_vector", dim = 3 }"#,
        r#"{ type = "f32_matrix", cols = 3 }"#
    )]
    field: &str,
) {
    assert!(coerced(field, Value::null()).as_null().is_some(), "{field}");
}

#[rstest]
#[case::text_from_non_utf8_bytes(r#"{ type = "text" }"#, Value::bytes(vec![0xff, 0xfe]), "not valid UTF-8")]
#[case::int_from_words(r#"{ type = "int" }"#, Value::string("x"), "cannot coerce to int")]
#[case::int_from_a_list(r#"{ type = "int" }"#, Value::list(vec![1_i64]), "cannot coerce to int")]
#[case::int_from_fractional_float(r#"{ type = "int" }"#, Value::f64(4.9), "cannot coerce to int")]
#[case::int_from_fractional_string(
    r#"{ type = "int" }"#,
    Value::string("3.7"),
    "cannot coerce to int"
)]
#[case::int_from_huge_u64(r#"{ type = "int" }"#, Value::u64(u64::MAX), "cannot coerce to int")]
#[case::int_list_from_fractional_floats(r#"{ type = "int_list" }"#, Value::list(vec![1.9_f64, 2.1]), "cannot coerce to int_list")]
#[case::int_list_from_huge_u64s(r#"{ type = "int_list" }"#, Value::list(vec![u64::MAX]), "cannot coerce to int_list")]
#[case::u8_vector_from_fractional_floats(r#"{ type = "u8_vector", dim = 3 }"#, Value::list(vec![1.9_f64, 2.9, 3.9]), "cannot coerce to u8_vector")]
#[case::u8_vector_from_negative(r#"{ type = "u8_vector", dim = 3 }"#, Value::list(vec![-1.0_f64, 2.0, 3.0]), "cannot coerce to u8_vector")]
#[case::u8_sparse_from_fraction(r#"{ type = "u8_sparse_vector" }"#, Value::r#struct([("7", Value::f64(3.5))]), "cannot coerce to u8_sparse_vector")]
#[case::float_from_words(r#"{ type = "float" }"#, Value::string("x"), "cannot coerce to float")]
#[case::bool_from_maybe(
    r#"{ type = "bool" }"#,
    Value::string("maybe"),
    "cannot coerce to bool"
)]
#[case::bool_from_a_float(r#"{ type = "bool" }"#, Value::f64(1.0), "cannot coerce to bool")]
#[case::bytes_from_a_string(
    r#"{ type = "bytes" }"#,
    Value::string("hello"),
    "cannot coerce to bytes"
)]
#[case::struct_from_a_json_array(
    r#"{ type = "struct" }"#,
    Value::string("[1, 2]"),
    "cannot coerce to struct"
)]
#[case::struct_from_an_int(r#"{ type = "struct" }"#, Value::i64(1), "cannot coerce to struct")]
#[case::int_list_from_strings(r#"{ type = "int_list" }"#, Value::list(vec!["a"]), "cannot coerce to int_list")]
#[case::float_list_from_strings(r#"{ type = "float_list" }"#, Value::list(vec!["a"]), "cannot coerce to float_list")]
#[case::text_list_from_ints(r#"{ type = "text_list" }"#, Value::list(vec![1_i64]), "cannot coerce to text_list")]
#[case::sparse_from_a_word_key(
    r#"{ type = "f32_sparse_vector" }"#,
    Value::string(r#"{"a": 1}"#),
    "cannot coerce to f32_sparse_vector"
)]
#[case::sparse_from_a_list(r#"{ type = "f32_sparse_vector" }"#, Value::list(vec![1.0_f32]), "cannot coerce to f32_sparse_vector")]
#[case::sparse_from_ragged_parallel_lists(
    r#"{ type = "f32_sparse_vector" }"#,
    Value::r#struct([("indices", Value::list(vec![1_i64, 2])), ("values", Value::list(vec![0.5_f64]))]),
    "sparse vector has 2 indices and 1 values"
)]
#[case::sparse_from_a_negative_index(
    r#"{ type = "f32_sparse_vector" }"#,
    Value::r#struct([("indices", Value::list(vec![-1_i64])), ("values", Value::list(vec![0.5_f64]))]),
    "sparse index -1 does not fit in a u32"
)]
#[case::vector_too_short(r#"{ type = "f32_vector", dim = 3 }"#, Value::list(vec![1.0_f32, 2.0]), "vector has 2 values, declared dim=3")]
#[case::vector_too_long(r#"{ type = "f32_vector", dim = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0]), "vector has 4 values, declared dim=3")]
#[case::vector_from_a_json_string(
    r#"{ type = "f32_vector", dim = 3 }"#,
    Value::string("\"abc\""),
    "cannot coerce to f32_vector"
)]
#[case::vector_from_garbage_text(
    r#"{ type = "f32_vector", dim = 3 }"#,
    Value::string("abc"),
    "expected value"
)]
#[case::packed_uneven(
    r#"{ type = "f32_vector", dim = 3 }"#,
    Value::bytes(vec![0; 8]),
    "8 bytes does not divide into dim=3"
)]
#[case::packed_width_three(
    r#"{ type = "f32_vector", dim = 3 }"#,
    Value::bytes(vec![0; 9]),
    "is 3 bytes per element"
)]
#[case::packed_float_width_into_a_byte_vector(
    r#"{ type = "u8_vector", dim = 3 }"#,
    Value::bytes(vec![0; 6]),
    "but u8_vector reads 1"
)]
#[case::packed_non_finite(
    r#"{ type = "f32_vector", dim = 1 }"#,
    le_f32(&[f32::INFINITY]),
    "non-finite float"
)]
#[case::vector_f32_overflows(r#"{ type = "f32_vector", dim = 1 }"#, Value::list(vec![1.0e300_f64]), "cannot coerce to f32_vector")]
#[case::vector_f16_overflows(r#"{ type = "f16_vector", dim = 1 }"#, Value::list(vec![1.0e6_f64]), "cannot coerce to f16_vector")]
#[case::vector_f8_overflows(r#"{ type = "f8_vector", dim = 1 }"#, Value::list(vec![1000.0_f64]), "cannot coerce to f8_vector")]
#[case::matrix_cols_mismatch(r#"{ type = "f32_matrix", cols = 3 }"#, Value::matrix(2, vec![1.0_f32, 2.0]), "matrix has 2 columns, declared cols=3")]
#[case::matrix_uneven(r#"{ type = "f32_matrix", cols = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0]), "4 values do not divide into cols=3")]
#[case::matrix_from_text(
    r#"{ type = "f32_matrix", cols = 3 }"#,
    Value::string("abc"),
    "expected value"
)]
fn refusals(#[case] field: &str, #[case] input: Value, #[case] fragment: &str) {
    let message = refused(coerce(field, input));
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::vector_without_dim(r#"{ type = "f32_vector" }"#, "requires `dim`")]
#[case::matrix_without_cols(r#"{ type = "f32_matrix" }"#, "requires `cols`")]
#[case::dim_on_a_scalar(r#"{ type = "text", dim = 3 }"#, "does not take `dim`")]
#[case::dim_on_a_sparse_vector(r#"{ type = "f32_sparse_vector", dim = 3 }"#, "does not take `dim`")]
#[case::cols_on_a_vector(
    r#"{ type = "f32_vector", dim = 3, cols = 3 }"#,
    "does not take `cols`"
)]
#[case::keyword_needs_text(
    r#"{ type = "int", index = "keyword" }"#,
    "a keyword index needs a `text` field"
)]
#[case::semantic_needs_text(
    r#"{ type = "int", index = "semantic" }"#,
    "a semantic index needs a `text` field"
)]
#[case::vector_index_needs_a_vector(
    r#"{ type = "text", index = { vector = { metric = "cosine" } } }"#,
    "a vector index needs a vector or sparse vector field"
)]
#[case::multi_vector_needs_a_matrix(
    r#"{ type = "text", index = { multi_vector = {} } }"#,
    "a multi_vector index needs a matrix field"
)]
fn field_validation(#[case] field: &str, #[case] fragment: &str) {
    let toml = spec_toml("f.parquet", "_id", &format!("v = {field}"));
    let message = refused(toml::from_str::<Spec>(&toml));
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::id_is_stringified(r#"title = { type = "text" }"#, vec![("_id", Value::i64(42)), ("title", Value::string("Dune"))], json!({"_id": "42", "title": "Dune"}))]
#[case::a_field_reads_another_column(r#"vec = { from = "embedding", type = "f32_vector", dim = 2 }"#, vec![("_id", Value::string("1")), ("embedding", Value::list(vec![1.0_f32, 2.0]))], json!({"_id": "1", "vec": [1.0, 2.0]}))]
#[case::undeclared_columns_are_dropped(r#"title = { type = "text" }"#, vec![("_id", Value::string("1")), ("title", Value::string("Dune")), ("extra", Value::i64(7))], json!({"_id": "1", "title": "Dune"}))]
fn documents(
    #[case] fields: &str,
    #[case] record: Vec<(&str, Value)>,
    #[case] expected: serde_json::Value,
) {
    let record = record
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    let doc = build_document(&target(fields), record).expect("document");
    assert_eq!(serde_json::Value::Object(json(&doc)), expected);
}

#[test]
fn custom_id_column() {
    let target = Target {
        id: Some("sku".to_string()),
        ..Default::default()
    };
    let record = vec![("sku".to_string(), Value::string("A-1"))];
    let doc = build_document(&target, record).expect("document");
    assert_eq!(doc.fields["_id"], Value::string("A-1"));
    assert!(!doc.fields.contains_key("sku"));
}

#[test]
fn required_field_missing() {
    let target = target(r#"title = { type = "text", required = true }"#);
    let record = vec![("_id".to_string(), Value::string("1"))];
    let message = refused(build_document(&target, record));
    assert!(
        message.contains("required field is missing"),
        "got: {message}"
    );
}

#[test]
fn oversized_document() {
    let oversized = target(r#"body = { type = "text" }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("body".to_string(), Value::string("x".repeat(200 * 1024))),
    ];
    let message = refused(build_document(&oversized, record));
    assert!(
        message.contains("exceeds the 200.0 KB document limit"),
        "got: {message}"
    );

    // The fix the message suggests.
    let truncated = target(r#"body = { type = "text", truncate = 100 }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("body".to_string(), Value::string("x".repeat(200 * 1024))),
    ];
    let doc = build_document(&truncated, record).expect("truncated document fits");
    assert_eq!(doc.fields["body"], Value::string("x".repeat(100)));
}

#[rstest]
#[case::null(vec![("_id".to_string(), Value::null())], "id is null")]
#[case::empty(vec![("_id".to_string(), Value::string(""))], "empty value cannot be a document id")]
#[case::absent(vec![("title".to_string(), Value::string("Dune")), ("author".to_string(), Value::string("Herbert"))], "which has: title, author")]
fn unusable_ids(#[case] record: Vec<(String, Value)>, #[case] fragment: &str) {
    let target = target(r#"title = { type = "text" }"#);
    let message = refused(build_document(&target, record));
    assert!(message.contains(fragment), "got: {message}");
}
