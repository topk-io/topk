use rstest::rstest;
use topk::import::{document, Error, Spec, Target};
use topk_rs::proto::v1::data::Value;

fn spec_toml(fields: &str) -> String {
    format!(
        r#"
[c]
from = "f.parquet"
id = "_id"

[c.fields]
{fields}
"#
    )
}

fn target(fields: &str) -> Target {
    let mut spec = Spec::parse(&spec_toml(fields)).expect("spec parses");
    spec.collections.shift_remove("c").expect("target c")
}

fn coerce(field: &str, value: Value) -> Result<Value, Error> {
    let target = target(&format!("v = {field}"));
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("v".to_string(), value),
    ];
    Ok(document(&target, record)?
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
    let doc = document(
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

#[rstest]
#[case::from_string(Value::string("hello"), "hello")]
#[case::from_bool(Value::bool(true), "true")]
#[case::from_int(Value::i64(42), "42")]
#[case::from_float(Value::f64(4.3), "4.3")]
#[case::from_utf8_bytes(Value::bytes(b"hello".to_vec()), "hello")]
#[case::from_struct(Value::r#struct([("a", Value::i64(1))]), r#"{"a":1}"#)]
fn text(#[case] input: Value, #[case] expected: &str) {
    assert_eq!(
        coerced(r#"{ type = "text" }"#, input),
        Value::string(expected)
    );
}

#[rstest]
#[case::cuts(Value::string("hello world"), "hello")]
#[case::shorter_stays(Value::string("hi"), "hi")]
#[case::char_boundary(Value::string("žluťoučký"), "žluťo")]
fn truncate(#[case] input: Value, #[case] expected: &str) {
    assert_eq!(
        coerced(r#"{ type = "text", truncate = 5 }"#, input),
        Value::string(expected)
    );
}

#[rstest]
#[case::from_int(Value::i64(42), 42)]
#[case::from_integral_float(Value::f64(4.0), 4)]
#[case::from_bool(Value::bool(true), 1)]
#[case::from_string(Value::string("42"), 42)]
#[case::from_padded_string(Value::string("  42  "), 42)]
#[case::from_negative_string(Value::string("-7"), -7)]
#[case::from_decimal_string(Value::string("3.00"), 3)]
fn int(#[case] input: Value, #[case] expected: i64) {
    assert_eq!(coerced(r#"{ type = "int" }"#, input), Value::i64(expected));
}

#[rstest]
#[case::from_int(Value::i64(1), 1.0)]
#[case::from_float(Value::f64(4.25), 4.25)]
#[case::from_string(Value::string(" 4.25 "), 4.25)]
fn float(#[case] input: Value, #[case] expected: f64) {
    assert_eq!(
        coerced(r#"{ type = "float" }"#, input),
        Value::f64(expected)
    );
}

#[rstest]
#[case::from_bool(Value::bool(true), true)]
#[case::from_nonzero_int(Value::i64(3), true)]
#[case::from_zero_int(Value::i64(0), false)]
#[case::from_true_word(Value::string("yes"), true)]
#[case::from_false_word(Value::string("no"), false)]
#[case::from_false_digit(Value::string("0"), false)]
fn bool(#[case] input: Value, #[case] expected: bool) {
    assert_eq!(
        coerced(r#"{ type = "bool" }"#, input),
        Value::bool(expected)
    );
}

#[test]
fn bytes() {
    let bytes = vec![0xde, 0xad, 0xbe, 0xef];
    assert_eq!(
        coerced(r#"{ type = "bytes" }"#, Value::bytes(bytes.clone())),
        Value::bytes(bytes)
    );
}

#[rstest]
#[case::int_list_from_ints("int_list", Value::list(vec![1_i64, 2]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_i32("int_list", Value::list(vec![1_i32, 2]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_integral_floats("int_list", Value::list(vec![1.0_f64, 2.0]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_json("int_list", Value::string("[1, 2]"), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_decimal_strings("int_list", Value::list(vec!["1.00", "2"]), Value::list(vec![1_i64, 2]))]
#[case::float_list_from_floats("float_list", Value::list(vec![0.5_f64, 1.5]), Value::list(vec![0.5_f32, 1.5]))]
#[case::float_list_from_ints("float_list", Value::list(vec![1_i64, 2]), Value::list(vec![1.0_f32, 2.0]))]
#[case::float_list_from_json("float_list", Value::string("[0.5, 1.5]"), Value::list(vec![0.5_f32, 1.5]))]
#[case::float_list_from_decimal_strings("float_list", Value::list(vec!["1.50", "2.25"]), Value::list(vec![1.5_f32, 2.25]))]
#[case::text_list_from_strings("text_list", Value::list(vec!["a", "b"]), Value::list(vec!["a", "b"]))]
#[case::text_list_from_json("text_list", Value::string(r#"["a", "b"]"#), Value::list(vec!["a", "b"]))]
fn lists(#[case] ty: &str, #[case] input: Value, #[case] expected: Value) {
    assert_eq!(coerced(&format!("{{ type = {ty:?} }}"), input), expected);
}

#[rstest]
#[case::f32("f32_vector", Value::list(vec![1_i64, 2, 3]), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::f32_from_pgvector_text("f32_vector", Value::string("[1,2,3]"), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::f32_from_json_floats("f32_vector", Value::string("[0.5, 1.5, 2.5]"), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::f32_from_floats("f32_vector", Value::list(vec![0.5_f64, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::f16("f16_vector", Value::list(vec![1_i64, 2, 3]), Value::list(vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0)]))]
#[case::f8("f8_vector", Value::list(vec![1_i64, 2, 3]), Value::list(vec![float8::F8E4M3::from_f32(1.0), float8::F8E4M3::from_f32(2.0), float8::F8E4M3::from_f32(3.0)]))]
#[case::u8("u8_vector", Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::u8_from_integral_floats("u8_vector", Value::list(vec![1.0_f64, 2.0, 3.0]), Value::list(vec![1_u8, 2, 3]))]
#[case::i8("i8_vector", Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_i8, 2, 3]))]
#[case::binary("binary_vector", Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
fn vectors(#[case] ty: &str, #[case] input: Value, #[case] expected: Value) {
    assert_eq!(
        coerced(&format!("{{ type = {ty:?}, dim = 3 }}"), input),
        expected
    );
}

/// A binary cell is a packed little-endian array; `dim` fixes the element
/// width, so the width is read off the data rather than the declared type.
#[rstest]
#[case::f32_from_f16_bytes("f32_vector", le_f16(&[1.0, 2.0, 3.0]), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::f32_from_f32_bytes("f32_vector", le_f32(&[0.5, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::f32_from_f64_bytes("f32_vector", le_f64(&[0.5, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::f16_from_f32_bytes("f16_vector", le_f32(&[1.0, 2.0, 3.0]), Value::list(vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0)]))]
#[case::u8_from_bytes("u8_vector", Value::bytes(vec![1, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::i8_from_signed_bytes("i8_vector", Value::bytes(vec![0xff, 2, 3]), Value::list(vec![-1_i8, 2, 3]))]
#[case::binary_from_bytes("binary_vector", Value::bytes(vec![1, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
fn packed_binary_vectors(#[case] ty: &str, #[case] input: Value, #[case] expected: Value) {
    assert_eq!(
        coerced(&format!("{{ type = {ty:?}, dim = 3 }}"), input),
        expected
    );
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

#[test]
fn matrices() {
    let input = Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(
        coerced(r#"{ type = "f32_matrix", cols = 3 }"#, input.clone()),
        input
    );
}

/// Multi-vector sources flatten their rows, so `cols` recovers the shape.
#[rstest]
#[case::f32("f32_matrix", Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]))]
#[case::f16("f16_matrix", Value::matrix(3, vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0), half::f16::from_f32(4.0), half::f16::from_f32(5.0), half::f16::from_f32(6.0)]))]
#[case::u8("u8_matrix", Value::matrix(3, vec![1_u8, 2, 3, 4, 5, 6]))]
#[case::i8("i8_matrix", Value::matrix(3, vec![1_i8, 2, 3, 4, 5, 6]))]
fn matrix_from_a_flat_list(#[case] ty: &str, #[case] expected: Value) {
    let flat = Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(
        coerced(&format!("{{ type = {ty:?}, cols = 3 }}"), flat),
        expected
    );
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
    let message = match document(&target, record) {
        Err(error) => error.to_string(),
        Ok(doc) => panic!("expected a refusal, got {doc:?}"),
    };
    assert!(message.contains("lost integer precision"), "got: {message}");
}

/// Sources that unify struct schemas across rows (duckdb reading jsonl) fill
/// absent keys with nulls; those are absent indices, not errors.
#[test]
fn sparse_skips_null_entries() {
    let input = Value::r#struct(vec![
        ("0".to_string(), Value::f64(1.0)),
        ("3".to_string(), Value::null()),
        ("7".to_string(), Value::f64(0.5)),
    ]);
    assert_eq!(
        coerced(r#"{ type = "f32_sparse_vector" }"#, input),
        Value::f32_sparse_vector(vec![0, 7], vec![1.0, 0.5]),
    );
}

/// A JSON string cell holding a flat list reshapes too.
#[test]
fn matrix_from_json_text() {
    let json = Value::string("[1.0, 2.0, 3.0, 4.0]");
    let expected = Value::matrix(2, vec![1.0_f32, 2.0, 3.0, 4.0]);
    assert_eq!(
        coerced(r#"{ type = "f32_matrix", cols = 2 }"#, json),
        expected
    );
}

#[rstest]
#[case::f32_from_json(r#"{ type = "f32_sparse_vector" }"#, Value::string(r#"{"3": 1.5, "1": 0.5}"#), Value::f32_sparse_vector(vec![1, 3], vec![0.5, 1.5]))]
#[case::f32_from_struct(r#"{ type = "f32_sparse_vector" }"#, Value::r#struct([("2", Value::f64(0.25))]), Value::f32_sparse_vector(vec![2], vec![0.25]))]
#[case::u8_from_ints(r#"{ type = "u8_sparse_vector" }"#, Value::r#struct([("7", Value::i64(3))]), Value::u8_sparse_vector(vec![7], vec![3]))]
fn sparse_vectors(#[case] field: &str, #[case] input: Value, #[case] expected: Value) {
    assert_eq!(coerced(field, input), expected);
}

#[test]
fn struct_from_json() {
    assert_eq!(
        coerced(
            r#"{ type = "struct" }"#,
            Value::string(r#"{"k": {"deep": 1}}"#)
        ),
        Value::r#struct([("k", Value::r#struct([("deep", Value::i64(1))]))])
    );
}

#[test]
fn struct_invalid_json() {
    let message = match coerce(r#"{ type = "struct" }"#, Value::string("not json")) {
        Err(error) => error.to_string(),
        Ok(value) => panic!("expected a refusal, got {value:?}"),
    };
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
#[case::matrix_cols_mismatch(r#"{ type = "f32_matrix", cols = 3 }"#, Value::matrix(2, vec![1.0_f32, 2.0]), "matrix has 2 columns, declared cols=3")]
#[case::matrix_uneven(r#"{ type = "f32_matrix", cols = 3 }"#, Value::list(vec![1.0_f32, 2.0, 3.0, 4.0]), "4 values do not divide into cols=3")]
#[case::matrix_from_text(
    r#"{ type = "f32_matrix", cols = 3 }"#,
    Value::string("abc"),
    "expected value"
)]
fn refusals(#[case] field: &str, #[case] input: Value, #[case] fragment: &str) {
    let message = match coerce(field, input) {
        Err(error) => error.to_string(),
        Ok(value) => panic!("expected a refusal, got {value:?}"),
    };
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
    let message = match Spec::parse(&spec_toml(&format!("v = {field}"))) {
        Err(Error::InvalidArgument(message)) => message,
        Err(other) => panic!("expected InvalidArgument, got {other:?}"),
        Ok(_) => panic!("expected {field} to be rejected"),
    };
    assert!(message.contains(fragment), "got: {message}");
}

#[test]
fn id_column() {
    let target = target(r#"title = { type = "text" }"#);
    let record = vec![
        ("_id".to_string(), Value::i64(42)),
        ("title".to_string(), Value::string("Dune")),
    ];
    let doc = document(&target, record).expect("document");
    assert_eq!(doc.fields["_id"], Value::string("42"));
    assert_eq!(doc.fields["title"], Value::string("Dune"));
}

#[test]
fn custom_id_column() {
    let target = Target {
        id: Some("sku".to_string()),
        ..Default::default()
    };
    let record = vec![("sku".to_string(), Value::string("A-1"))];
    let doc = document(&target, record).expect("document");
    assert_eq!(doc.fields["_id"], Value::string("A-1"));
    assert!(!doc.fields.contains_key("sku"));
}

#[test]
fn field_from() {
    let target = target(r#"vec = { from = "embedding", type = "f32_vector", dim = 2 }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("embedding".to_string(), Value::list(vec![1.0_f32, 2.0])),
    ];
    let doc = document(&target, record).expect("document");
    assert_eq!(doc.fields["vec"], Value::list(vec![1.0_f32, 2.0]));
    assert!(!doc.fields.contains_key("embedding"));
}

#[test]
fn undeclared_columns_are_dropped() {
    let target = target(r#"title = { type = "text" }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("title".to_string(), Value::string("Dune")),
        ("extra".to_string(), Value::i64(7)),
    ];
    let doc = document(&target, record).expect("document");
    assert_eq!(doc.fields["title"], Value::string("Dune"));
    assert!(!doc.fields.contains_key("extra"));
}

#[test]
fn required_field_missing() {
    let target = target(r#"title = { type = "text", required = true }"#);
    let record = vec![("_id".to_string(), Value::string("1"))];
    let message = match document(&target, record) {
        Err(error) => error.to_string(),
        Ok(doc) => panic!("expected a refusal, got {doc:?}"),
    };
    assert!(
        message.contains("required field is missing"),
        "got: {message}"
    );
}

/// `truncate` is text-only, so a binary-dominated document must be told
/// something it can act on.
#[test]
fn oversized_binary_document() {
    let target = target(r#"blob = { type = "bytes" }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("blob".to_string(), Value::bytes(vec![0u8; 250 * 1024])),
    ];
    let message = match document(&target, record) {
        Err(error) => error.to_string(),
        Ok(doc) => panic!("expected a refusal, got {doc:?}"),
    };
    assert!(
        message.contains("binary field cannot be truncated"),
        "got: {message}"
    );
    assert!(!message.contains("`truncate = <chars>`"), "got: {message}");
}

#[test]
fn oversized_document() {
    let oversized = target(r#"body = { type = "text" }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("body".to_string(), Value::string("x".repeat(200 * 1024))),
    ];
    let message = match document(&oversized, record) {
        Err(error) => error.to_string(),
        Ok(doc) => panic!("expected a refusal, got {doc:?}"),
    };
    assert!(
        message.contains("over the 195.3 kiB limit"),
        "got: {message}"
    );
    assert!(
        message.contains("body"),
        "names the biggest field: {message}"
    );
    assert!(message.contains("truncate"), "points at the fix: {message}");

    // The fix the message suggests.
    let truncated = target(r#"body = { type = "text", truncate = 100 }"#);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("body".to_string(), Value::string("x".repeat(200 * 1024))),
    ];
    let doc = document(&truncated, record).expect("truncated document fits");
    assert_eq!(doc.fields["body"], Value::string("x".repeat(100)));
}

#[rstest]
#[case::null(vec![("_id".to_string(), Value::null())], "id is null")]
#[case::empty(vec![("_id".to_string(), Value::string(""))], "empty value cannot be a document id")]
#[case::absent(vec![("title".to_string(), Value::string("Dune")), ("author".to_string(), Value::string("Herbert"))], "which has: title, author")]
fn unusable_ids(#[case] record: Vec<(String, Value)>, #[case] fragment: &str) {
    let target = target(r#"title = { type = "text" }"#);
    let message = match document(&target, record) {
        Err(error) => error.to_string(),
        Ok(doc) => panic!("expected a refusal, got {doc:?}"),
    };
    assert!(message.contains(fragment), "got: {message}");
}
