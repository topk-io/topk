use rstest::rstest;
use topk::import::spec::{Field, Target};
use topk_rs::doc;
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{Document, Value};

use crate::import::common::refused;

fn row<const N: usize>(fields: [(&str, Field); N]) -> Target {
    Target {
        from: "unused".into(),
        id: "_id".into(),
        fields: fields
            .into_iter()
            .map(|(name, field)| (name.into(), field))
            .collect(),
        filter: None,
        partition: None,
        limit: None,
    }
}

/// One column may feed several fields — the id column included.
#[test]
fn fields_share_columns_and_the_id() {
    let target = row([
        (
            "sku",
            Field {
                from: Some("_id".into()),
                ..Field::new(FieldSpec::text(false))
            },
        ),
        (
            "a",
            Field {
                from: Some("v".into()),
                ..Field::new(FieldSpec::integer(false))
            },
        ),
        (
            "b",
            Field {
                from: Some("v".into()),
                ..Field::new(FieldSpec::float(false))
            },
        ),
    ]);
    let (_, doc) = target
        .document(vec![
            ("_id".to_string(), Value::string("1")),
            ("v".to_string(), Value::i64(7)),
        ])
        .expect("document");
    assert_eq!(
        doc,
        doc!("_id" => "1", "sku" => "1", "a" => 7_i64, "b" => 7.0_f64)
    );
}

/// Every accepted (declared field, source cell) pair. A binary cell is a packed
/// little-endian array whose element width follows from `dim`, not the declared
/// type; a JSON string cell reshapes into any container.
#[rstest]
#[case::text_from_string(FieldSpec::text(false), Value::string("hello"), Value::string("hello"))]
#[case::text_from_bool(FieldSpec::text(false), Value::bool(true), Value::string("true"))]
#[case::text_from_int(FieldSpec::text(false), Value::i64(42), Value::string("42"))]
#[case::text_from_float(FieldSpec::text(false), Value::f64(4.3), Value::string("4.3"))]
#[case::text_from_utf8_bytes(FieldSpec::text(false), Value::bytes(b"hello".to_vec()), Value::string("hello"))]
#[case::text_from_struct(FieldSpec::text(false), Value::r#struct([("a", Value::i64(1))]), Value::string(r#"{"a":1}"#))]
#[case::int_from_int(FieldSpec::integer(false), Value::i64(42), Value::i64(42))]
#[case::int_from_integral_float(FieldSpec::integer(false), Value::f64(4.0), Value::i64(4))]
#[case::int_at_lower_bound(FieldSpec::integer(false), Value::f64(i64::MIN as f64), Value::i64(i64::MIN))]
#[case::int_below_upper_bound(FieldSpec::integer(false), Value::f64(((1_u64 << 63) - 1024) as f64), Value::i64(i64::MAX - 1023))]
#[case::int_from_bool(FieldSpec::integer(false), Value::bool(true), Value::i64(1))]
#[case::int_from_string(FieldSpec::integer(false), Value::string("42"), Value::i64(42))]
#[case::int_from_padded_string(FieldSpec::integer(false), Value::string("  42  "), Value::i64(42))]
#[case::int_from_negative_string(FieldSpec::integer(false), Value::string("-7"), Value::i64(-7))]
#[case::int_from_decimal_string(FieldSpec::integer(false), Value::string("3.00"), Value::i64(3))]
#[case::float_from_int(FieldSpec::float(false), Value::i64(1), Value::f64(1.0))]
#[case::float_from_float(FieldSpec::float(false), Value::f64(4.25), Value::f64(4.25))]
#[case::float_from_padded_string(
    FieldSpec::float(false),
    Value::string(" 4.25 "),
    Value::f64(4.25)
)]
#[case::bool_from_bool(FieldSpec::boolean(false), Value::bool(true), Value::bool(true))]
#[case::bool_from_nonzero_int(FieldSpec::boolean(false), Value::i64(3), Value::bool(true))]
#[case::bool_from_zero_int(FieldSpec::boolean(false), Value::i64(0), Value::bool(false))]
#[case::bool_from_true_word(FieldSpec::boolean(false), Value::string("yes"), Value::bool(true))]
#[case::bool_from_false_word(FieldSpec::boolean(false), Value::string("no"), Value::bool(false))]
#[case::bool_from_false_digit(FieldSpec::boolean(false), Value::string("0"), Value::bool(false))]
#[case::bytes_stay_bytes(FieldSpec::bytes(false), Value::bytes(vec![0xde, 0xad]), Value::bytes(vec![0xde, 0xad]))]
#[case::struct_from_json(FieldSpec::r#struct(false, Vec::<(String, FieldSpec)>::new()), Value::string(r#"{"k": {"deep": 1}}"#), Value::r#struct([("k", Value::r#struct([("deep", Value::i64(1))]))]))]
#[case::int_list_from_ints(FieldSpec::list(false, ListValueType::Integer), Value::list(vec![1_i64, 2]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_i32(FieldSpec::list(false, ListValueType::Integer), Value::list(vec![1_i32, 2]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_integral_floats(FieldSpec::list(false, ListValueType::Integer), Value::list(vec![1.0_f64, 2.0]), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_json(FieldSpec::list(false, ListValueType::Integer), Value::string("[1, 2]"), Value::list(vec![1_i64, 2]))]
#[case::int_list_from_decimal_strings(FieldSpec::list(false, ListValueType::Integer), Value::list(vec!["1.00", "2"]), Value::list(vec![1_i64, 2]))]
#[case::float_list_from_floats(FieldSpec::list(false, ListValueType::Float), Value::list(vec![0.5_f64, 1.5]), Value::list(vec![0.5_f32, 1.5]))]
#[case::float_list_from_ints(FieldSpec::list(false, ListValueType::Float), Value::list(vec![1_i64, 2]), Value::list(vec![1.0_f32, 2.0]))]
#[case::float_list_from_json(FieldSpec::list(false, ListValueType::Float), Value::string("[0.5, 1.5]"), Value::list(vec![0.5_f32, 1.5]))]
#[case::float_list_from_decimal_strings(FieldSpec::list(false, ListValueType::Float), Value::list(vec!["1.50", "2.25"]), Value::list(vec![1.5_f32, 2.25]))]
#[case::text_list_from_strings(FieldSpec::list(false, ListValueType::String), Value::list(vec!["a", "b"]), Value::list(vec!["a", "b"]))]
#[case::text_list_from_json(FieldSpec::list(false, ListValueType::String), Value::string(r#"["a", "b"]"#), Value::list(vec!["a", "b"]))]
#[case::vector_f32(FieldSpec::f32_vector(3, false), Value::list(vec![1_i64, 2, 3]), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::vector_f32_from_pgvector_text(FieldSpec::f32_vector(3, false), Value::string("[1,2,3]"), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::vector_f32_from_json_floats(FieldSpec::f32_vector(3, false), Value::string("[0.5, 1.5, 2.5]"), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::vector_f32_from_floats(FieldSpec::f32_vector(3, false), Value::list(vec![0.5_f64, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::vector_f16(FieldSpec::f16_vector(3, false), Value::list(vec![1_i64, 2, 3]), Value::list(vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0)]))]
#[case::vector_f8(FieldSpec::f8_vector(3, false), Value::list(vec![1_i64, 2, 3]), Value::list(vec![float8::F8E4M3::from_f32(1.0), float8::F8E4M3::from_f32(2.0), float8::F8E4M3::from_f32(3.0)]))]
#[case::vector_u8(FieldSpec::u8_vector(3, false), Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::vector_u8_from_integral_floats(FieldSpec::u8_vector(3, false), Value::list(vec![1.0_f64, 2.0, 3.0]), Value::list(vec![1_u8, 2, 3]))]
#[case::vector_i8(FieldSpec::i8_vector(3, false), Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_i8, 2, 3]))]
#[case::vector_binary(FieldSpec::binary_vector(3, false), Value::list(vec![1_i64, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::vector_f16_at_max(FieldSpec::f16_vector(1, false), Value::list(vec![65504.0_f64]), Value::list(vec![half::f16::MAX]))]
#[case::vector_f8_in_range(FieldSpec::f8_vector(1, false), Value::list(vec![400.0_f64]), Value::list(vec![float8::F8E4M3::from_f64(400.0)]))]
#[case::packed_f32_from_f16_bytes(FieldSpec::f32_vector(3, false), le_f16(&[1.0, 2.0, 3.0]), Value::list(vec![1.0_f32, 2.0, 3.0]))]
#[case::packed_f32_from_f32_bytes(FieldSpec::f32_vector(3, false), le_f32(&[0.5, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::packed_f32_from_f64_bytes(FieldSpec::f32_vector(3, false), le_f64(&[0.5, 1.5, 2.5]), Value::list(vec![0.5_f32, 1.5, 2.5]))]
#[case::packed_f16_from_f32_bytes(FieldSpec::f16_vector(3, false), le_f32(&[1.0, 2.0, 3.0]), Value::list(vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0)]))]
#[case::packed_u8_from_bytes(FieldSpec::u8_vector(3, false), Value::bytes(vec![1, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::packed_i8_from_signed_bytes(FieldSpec::i8_vector(3, false), Value::bytes(vec![0xff, 2, 3]), Value::list(vec![-1_i8, 2, 3]))]
#[case::packed_binary_from_bytes(FieldSpec::binary_vector(3, false), Value::bytes(vec![1, 2, 3]), Value::list(vec![1_u8, 2, 3]))]
#[case::matrix_stays_a_matrix(FieldSpec::matrix(false, 3, MatrixValueType::F32), Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]))]
#[case::matrix_f32_from_a_flat_list(FieldSpec::matrix(false, 3, MatrixValueType::F32), Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]))]
#[case::matrix_f16_from_a_flat_list(FieldSpec::matrix(false, 3, MatrixValueType::F16), Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![half::f16::from_f32(1.0), half::f16::from_f32(2.0), half::f16::from_f32(3.0), half::f16::from_f32(4.0), half::f16::from_f32(5.0), half::f16::from_f32(6.0)]))]
#[case::matrix_u8_from_a_flat_list(FieldSpec::matrix(false, 3, MatrixValueType::U8), Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1_u8, 2, 3, 4, 5, 6]))]
#[case::matrix_i8_from_a_flat_list(FieldSpec::matrix(false, 3, MatrixValueType::I8), Value::list(vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0]), Value::matrix(3, vec![1_i8, 2, 3, 4, 5, 6]))]
#[case::matrix_from_json_text(FieldSpec::matrix(false, 2, MatrixValueType::F32), Value::string("[1.0, 2.0, 3.0, 4.0]"), Value::matrix(2, vec![1.0_f32, 2.0, 3.0, 4.0]))]
#[case::sparse_f32_from_json(FieldSpec::f32_sparse_vector(false), Value::string(r#"{"3": 1.5, "1": 0.5}"#), Value::sparse_vector(vec![1, 3], vec![0.5, 1.5]))]
#[case::sparse_f32_from_struct(FieldSpec::f32_sparse_vector(false), Value::r#struct([("2", Value::f64(0.25))]), Value::sparse_vector(vec![2], vec![0.25]))]
#[case::sparse_u8_from_ints(FieldSpec::u8_sparse_vector(false), Value::r#struct([("7", Value::i64(3))]), Value::sparse_vector(vec![7], vec![3u8]))]
#[case::sparse_skips_null_entries(FieldSpec::f32_sparse_vector(false), Value::r#struct([("0", Value::f64(1.0)), ("3", Value::null()), ("7", Value::f64(0.5))]), Value::sparse_vector(vec![0, 7], vec![1.0, 0.5]))]
#[case::sparse_f32_from_parallel_lists(FieldSpec::f32_sparse_vector(false), Value::r#struct([("indices", Value::list(vec![3_i64, 1])), ("values", Value::list(vec![1.5_f64, 0.5]))]), Value::sparse_vector(vec![1, 3], vec![0.5, 1.5]))]
#[case::sparse_u8_from_parallel_lists(FieldSpec::u8_sparse_vector(false), Value::r#struct([("indices", Value::list(vec![7_i64])), ("values", Value::list(vec![3_i64]))]), Value::sparse_vector(vec![7], vec![3u8]))]
#[case::sparse_from_empty_parallel_lists(FieldSpec::f32_sparse_vector(false), Value::r#struct([("indices", Value::list(Vec::<i64>::new())), ("values", Value::list(Vec::<f64>::new()))]), Value::sparse_vector(Vec::<u32>::new(), Vec::<f32>::new()))]
fn coerces(#[case] schema: FieldSpec, #[case] input: Value, #[case] expected: Value) {
    assert_eq!(Field::new(schema).coerce(input).expect("coerces"), expected);
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
    let target = row([("v", Field::new(FieldSpec::integer(false)))]);
    let record = vec![
        (
            "_id".to_string(),
            Value::f64(18446744073709551615_u64 as f64),
        ),
        ("v".to_string(), Value::i64(1)),
    ];
    let message = refused(target.document(record));
    assert!(message.contains("lost integer precision"), "got: {message}");
}

#[rstest]
#[case::large_digits("999999999999999999999999")]
#[case::looks_like_scientific("1e999")]
fn numeric_looking_string_id_is_preserved(#[case] id: &str) {
    let (_, doc) = (row([("v", Field::new(FieldSpec::integer(false)))]))
        .document(vec![
            ("_id".to_string(), Value::string(id)),
            ("v".to_string(), Value::i64(1)),
        ])
        .unwrap();
    assert_eq!(doc.fields.get("_id"), Some(&Value::string(id)));
}

#[test]
fn struct_invalid_json() {
    let message = refused(
        (row([(
            "v",
            Field::new(FieldSpec::r#struct(
                false,
                Vec::<(String, FieldSpec)>::new(),
            )),
        )]))
        .document(vec![
            ("_id".into(), Value::string("1")),
            ("v".into(), Value::string("not json")),
        ]),
    );
    assert!(message.contains(r#"doc "1""#), "got: {message}");
    assert!(message.contains(r#"field "v""#), "got: {message}");
}

#[rstest]
fn nulls(
    #[values(
        Field::new(FieldSpec::text(false)),
        Field::new(FieldSpec::f32_vector(3, false)),
        Field::new(FieldSpec::matrix(false, 3, MatrixValueType::F32))
    )]
    field: Field,
) {
    assert!(field.coerce(Value::null()).unwrap().as_null().is_some());
}

#[rstest]
#[case::text_from_non_utf8_bytes(FieldSpec::text(false), Value::bytes(vec![0xff, 0xfe]), "not valid UTF-8")]
#[case::int_from_words(FieldSpec::integer(false), Value::string("x"), " to int")]
#[case::int_from_a_list(FieldSpec::integer(false), Value::list(vec![1_i64]), " to int")]
#[case::int_from_fractional_float(FieldSpec::integer(false), Value::f64(4.9), " to int")]
#[case::int_from_fractional_string(FieldSpec::integer(false), Value::string("3.7"), " to int")]
#[case::int_at_upper_bound(FieldSpec::integer(false), Value::f64((1_u64 << 63) as f64), " to int")]
#[case::int_list_at_upper_bound(FieldSpec::list(false, ListValueType::Integer), Value::list(vec![(1_u64 << 63) as f64]), " to int_list")]
#[case::int_from_huge_u64(FieldSpec::integer(false), Value::u64(u64::MAX), " to int")]
#[case::int_list_from_fractional_floats(FieldSpec::list(false, ListValueType::Integer), Value::list(vec![1.9_f64, 2.1]), " to int_list")]
#[case::int_list_from_huge_u64s(FieldSpec::list(false, ListValueType::Integer), Value::list(vec![u64::MAX]), " to int_list")]
#[case::u8_vector_from_fractional_floats(FieldSpec::u8_vector(3, false), Value::list(vec![1.9_f64, 2.9, 3.9]), " to u8_vector")]
#[case::u8_vector_from_negative(FieldSpec::u8_vector(3, false), Value::list(vec![-1.0_f64, 2.0, 3.0]), " to u8_vector")]
#[case::matrix_from_fraction(FieldSpec::matrix(false, 2, MatrixValueType::U8), Value::matrix(2, vec![1.5_f32, 2.0]), " to u8_matrix")]
#[case::matrix_from_negative(FieldSpec::matrix(false, 2, MatrixValueType::U8), Value::matrix(2, vec![-1.0_f32, 2.0]), " to u8_matrix")]
#[case::matrix_from_overflow(FieldSpec::matrix(false, 2, MatrixValueType::I8), Value::matrix(2, vec![128.0_f32, 2.0]), " to i8_matrix")]
#[case::u8_sparse_from_fraction(FieldSpec::u8_sparse_vector(false), Value::r#struct([("7", Value::f64(3.5))]), " to u8_sparse_vector")]
#[case::float_from_words(FieldSpec::float(false), Value::string("x"), " to float")]
#[case::bool_from_maybe(FieldSpec::boolean(false), Value::string("maybe"), " to bool")]
#[case::bool_from_a_float(FieldSpec::boolean(false), Value::f64(1.0), " to bool")]
#[case::bytes_from_a_string(FieldSpec::bytes(false), Value::string("hello"), " to bytes")]
#[case::struct_from_a_json_array(FieldSpec::r#struct(false, Vec::<(String, FieldSpec)>::new()), Value::string("[1, 2]"), " to struct")]
#[case::struct_from_an_int(FieldSpec::r#struct(false, Vec::<(String, FieldSpec)>::new()), Value::i64(1), " to struct")]
#[case::int_list_from_strings(FieldSpec::list(false, ListValueType::Integer), Value::list(vec!["a"]), " to int_list")]
#[case::float_list_from_strings(FieldSpec::list(false, ListValueType::Float), Value::list(vec!["a"]), " to float_list")]
#[case::text_list_from_ints(FieldSpec::list(false, ListValueType::String), Value::list(vec![1_i64]), " to text_list")]
#[case::sparse_from_a_word_key(
    FieldSpec::f32_sparse_vector(false),
    Value::string(r#"{"a": 1}"#),
    " to f32_sparse_vector"
)]
#[case::sparse_from_a_list(FieldSpec::f32_sparse_vector(false), Value::list(vec![1.0_f32]), " to f32_sparse_vector")]
#[case::sparse_from_ragged_parallel_lists( FieldSpec::f32_sparse_vector(false), Value::r#struct([("indices", Value::list(vec![1_i64, 2])), ("values", Value::list(vec![0.5_f64]))]),
    "sparse vector has 2 indices and 1 values"
)]
#[case::sparse_from_a_negative_index( FieldSpec::f32_sparse_vector(false), Value::r#struct([("indices", Value::list(vec![-1_i64])), ("values", Value::list(vec![0.5_f64]))]),
    " to f32_sparse_vector"
)]
#[case::vector_too_short(FieldSpec::f32_vector(3, false), Value::list(vec![1.0_f32, 2.0]), "vector has 2 values, declared dim=3")]
#[case::vector_too_long(FieldSpec::f32_vector(3, false), Value::list(vec![1.0_f32, 2.0, 3.0, 4.0]), "vector has 4 values, declared dim=3")]
#[case::vector_from_a_json_string(
    FieldSpec::f32_vector(3, false),
    Value::string("\"abc\""),
    " to f32_vector"
)]
#[case::vector_from_garbage_text(
    FieldSpec::f32_vector(3, false),
    Value::string("abc"),
    "expected value"
)]
#[case::packed_uneven( FieldSpec::f32_vector(3, false), Value::bytes(vec![0; 8]), "8 bytes does not divide into dim=3" )]
#[case::packed_width_three( FieldSpec::f32_vector(3, false), Value::bytes(vec![0; 9]), "is 3 bytes per element" )]
#[case::packed_float_width_into_a_byte_vector( FieldSpec::u8_vector(3, false), Value::bytes(vec![0; 6]), "but u8_vector reads 1" )]
#[case::packed_non_finite( FieldSpec::f32_vector(1, false), le_f32(&[f32::INFINITY]), "non-finite float" )]
#[case::vector_f32_overflows(FieldSpec::f32_vector(1, false), Value::list(vec![1.0e300_f64]), " to f32_vector")]
#[case::vector_f16_overflows(FieldSpec::f16_vector(1, false), Value::list(vec![1.0e6_f64]), " to f16_vector")]
#[case::vector_f8_overflows(FieldSpec::f8_vector(1, false), Value::list(vec![1000.0_f64]), " to f8_vector")]
#[case::matrix_cols_mismatch(FieldSpec::matrix(false, 3, MatrixValueType::F32), Value::matrix(2, vec![1.0_f32, 2.0]), "matrix has 2 columns, declared cols=3")]
#[case::matrix_uneven(FieldSpec::matrix(false, 3, MatrixValueType::F32), Value::list(vec![1.0_f32, 2.0, 3.0, 4.0]), "4 values do not divide into cols=3")]
#[case::matrix_from_text(
    FieldSpec::matrix(false, 3, MatrixValueType::F32),
    Value::string("abc"),
    "expected value"
)]
fn refusals(#[case] schema: FieldSpec, #[case] input: Value, #[case] fragment: &str) {
    let message = refused(Field::new(schema).coerce(input));
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::id_is_stringified([("title", Field::new(FieldSpec::text(false)))], vec![("_id", Value::i64(42)), ("title", Value::string("Dune"))], doc!("_id" => "42", "title" => "Dune"))]
#[case::a_field_reads_another_column([("vec", Field { from: Some("embedding".into()), ..Field::new(FieldSpec::f32_vector(2, false)) })], vec![("_id", Value::string("1")), ("embedding", Value::list(vec![1.0_f32, 2.0]))], doc!("_id" => "1", "vec" => Value::list(vec![1.0_f32, 2.0])))]
#[case::undeclared_columns_are_dropped([("title", Field::new(FieldSpec::text(false)))], vec![("_id", Value::string("1")), ("title", Value::string("Dune")), ("extra", Value::i64(7))], doc!("_id" => "1", "title" => "Dune"))]
fn documents(
    #[case] fields: [(&str, Field); 1],
    #[case] record: Vec<(&str, Value)>,
    #[case] expected: Document,
) {
    let record = record
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    let (_, doc) = (row(fields)).document(record).expect("document");
    assert_eq!(doc, expected);
}

#[test]
fn custom_id_column() {
    let target = Target {
        id: "sku".to_string(),
        ..row([])
    };
    let record = vec![("sku".to_string(), Value::string("A-1"))];
    let (_, doc) = target.document(record).expect("document");
    assert_eq!(doc, doc!("_id" => "A-1"));
}

#[test]
fn required_field_missing() {
    let target = row([(
        "title",
        Field {
            from: None,
            ..Field::new(FieldSpec::text(true))
        },
    )]);
    let record = vec![("_id".to_string(), Value::string("1"))];
    let message = refused(target.document(record));
    assert!(
        message.contains("required field is missing"),
        "got: {message}"
    );
}

#[test]
fn oversized_document() {
    let oversized = row([("body", Field::new(FieldSpec::text(false)))]);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("body".to_string(), Value::string("x".repeat(2_000_000))),
    ];
    let message = refused(oversized.document(record));
    assert_eq!(
        message.matches("exceeds the 2.0 MB document limit").count(),
        1,
        "{message}"
    );
    assert!(message.contains("largest field \"body\""), "got: {message}");

    // The fix the message suggests.
    let truncated = row([(
        "body",
        Field {
            truncate: Some(100),
            ..Field::new(FieldSpec::text(false))
        },
    )]);
    let record = vec![
        ("_id".to_string(), Value::string("1")),
        ("body".to_string(), Value::string("x".repeat(2_000_000))),
    ];
    let (_, doc) = truncated.document(record).expect("truncated document fits");
    assert_eq!(doc.fields["body"], Value::string("x".repeat(100)));
}

#[rstest]
#[case::null(vec![("_id".to_string(), Value::null())], "id is null")]
#[case::empty(vec![("_id".to_string(), Value::string(""))], "empty value cannot be a document id")]
#[case::absent(vec![("title".to_string(), Value::string("Dune")), ("author".to_string(), Value::string("Herbert"))], "which has: title, author")]
fn unusable_ids(#[case] record: Vec<(String, Value)>, #[case] fragment: &str) {
    let target = row([("title", Field::new(FieldSpec::text(false)))]);
    let message = refused(target.document(record));
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::packed_zero(Field::new(FieldSpec::f32_vector(0, false)), Value::bytes(vec![0; 4]))]
#[case::matrix_zero(Field::new(FieldSpec::matrix(false, 0, MatrixValueType::F32)), Value::list(vec![1.0f32]))]
fn zero_shape_does_not_panic(#[case] ty: Field, #[case] value: Value) {
    assert!(ty.coerce(value).is_err());
}

#[rstest]
#[case::nested_f16(MatrixValueType::F16, Value::matrix(2, [1., 2., 3., 4.].map(half::f16::from_f64).to_vec()))]
#[case::nested_f8(MatrixValueType::F8, Value::matrix(2, [1., 2., 3., 4.].map(float8::F8E4M3::from_f64).to_vec()))]
#[case::nested_u8(MatrixValueType::U8, Value::matrix(2, vec![1_u8, 2, 3, 4]))]
#[case::nested_i8(MatrixValueType::I8, Value::matrix(2, vec![1_i8, 2, 3, 4]))]
fn nested_matrices(#[case] ty: MatrixValueType, #[case] expected: Value) {
    let field = Field::new(FieldSpec::matrix(false, 2, ty));
    assert_eq!(
        field.coerce(Value::string("[[1,2],[3,4]]")).unwrap(),
        expected
    );
}

#[rstest]
#[case::empty(0, "hello", "")]
#[case::shorter(10, "hello", "hello")]
#[case::cuts(3, "hello", "hel")]
#[case::unicode(2, "ééé", "éé")]
fn truncates(#[case] limit: usize, #[case] input: &str, #[case] expected: &str) {
    let field = Field {
        truncate: Some(limit),
        ..Field::new(FieldSpec::text(false))
    };
    assert_eq!(
        field.coerce(Value::string(input)).unwrap(),
        Value::string(expected)
    );
}
