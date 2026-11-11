use crate::import::common::{refused, target};
use rstest::rstest;
use topk::import::source::Uri;
use topk::import::spec::{Field, Spec, Target};
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::control::{
    field_index, FieldIndex, FieldSpec, FieldType, KeywordIndexType, MultiVectorDistanceMetric,
    MultiVectorIndex, MultiVectorQuantization, SemanticIndex, VectorDistanceMetric, VectorIndex,
};

#[rstest]
#[case::leading_underscore("_books", "collection names start with a letter or digit")]
#[case::leading_hyphen("-books", "collection names start with a letter or digit")]
#[case::leading_dot(".books", "collection names start with a letter or digit")]
#[case::inner_space("my books", "collection names start with a letter or digit")]
fn invalid_collection_names(#[case] name: &str, #[case] fragment: &str) {
    let message = refused(Spec::try_from(indexmap::IndexMap::from([(
        name.into(),
        target("f.parquet", "_id", [("t", FieldSpec::text(false))]),
    )])));
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::mixed_case("myBooks")]
#[case::leading_digit("2026-logs")]
#[case::dotted("logs-2026.07.29")]
#[case::digits_and_separators("b9_books-v2")]
fn valid_collection_names(#[case] name: &str) {
    let spec = Spec::try_from(indexmap::IndexMap::from([(
        name.into(),
        target("f.parquet", "_id", [("t", FieldSpec::text(false))]),
    )]))
    .unwrap();
    assert!(spec.collections.contains_key(name), "{name}");
}

#[rstest]
#[case::empty_from("", "t", "`from` is empty")]
#[case::blank_from("   ", "t", "`from` is empty")]
#[case::reserved_id("f.parquet", "_id", "cannot be empty or start with `_`")]
#[case::reserved_prefix("f.parquet", "_score", "cannot be empty or start with `_`")]
fn invalid_targets(#[case] from: &str, #[case] field: &str, #[case] fragment: &str) {
    let message = refused(target(from, "_id", [(field, FieldSpec::text(false))]).validate());
    assert!(message.contains(fragment), "got: {message}");
}

#[test]
fn toml_invokes_field_validation() {
    let message = refused(toml::from_str::<Spec>(
        "[c]\nfrom = 'f.parquet'\n[c.fields]\nv = { type = 'text', index = { vector = { metric = 'cosine' } } }",
    ));
    assert!(message.contains("incompatible index"), "got: {message}");
}

#[rstest]
#[case::missing_from(
    "[c]\nid = '_id'\n[c.fields]\nt = { type = 'text' }",
    "missing field `from`"
)]
#[case::unknown_target_key(
    "[c]\nfrom = 'f.parquet'\nbogus = 1\n[c.fields]\nt = { type = 'text' }",
    "unknown field `bogus`"
)]
fn unknown_keys(#[case] toml: &str, #[case] fragment: &str) {
    let message = refused(toml::from_str::<Spec>(toml));
    assert!(message.contains(fragment), "got: {message}");
}

#[test]
fn minimal_target() {
    let spec = toml::from_str::<Spec>(
        r#"
[c]
from = "books.parquet"

[c.fields]
title = { type = "text" }
"#,
    )
    .expect("spec parses");
    let target = &spec.collections["c"];
    assert_eq!(target.from, "books.parquet");
    assert_eq!(target.id, "_id");
    assert_eq!(target.fields.len(), 1);
}

#[rstest]
#[case::missing_fields("", "missing field `fields`")]
#[case::empty_fields("[c.fields]", "declare at least one field")]
fn no_fields_is_rejected(#[case] fields: &str, #[case] expected: &str) {
    let message = refused(toml::from_str::<Spec>(&format!(
        "[c]\nfrom = 'books.parquet'\n{fields}"
    )));
    assert!(message.contains(expected), "{message}");
}

#[test]
fn serialize_round_trip() {
    let toml = r#"
[zebra]
from = "z.parquet"
[zebra.fields]
t = { type = "text" }

[books]
from = "public.books"
id = "sku"
filter = "published_year > 1950"
limit = 10

[books.fields]
title = { type = "text", required = true, index = "keyword" }
embedding = { from = "vec", type = "f32_vector", dim = 3, index = { vector = { metric = "cosine" } } }
page_counts = { type = "int_list" }
"#;
    let spec = toml::from_str::<Spec>(toml).unwrap();
    assert_eq!(
        spec.collections
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["zebra", "books"]
    );
    assert_eq!(spec.collections["books"].id, "sku");
    assert_eq!(
        spec.collections["books"].filter.as_deref(),
        Some("published_year > 1950")
    );
    assert_eq!(spec.collections["books"].limit, Some(10));
    let printed = toml::to_string_pretty(&spec).unwrap();
    let reparsed = toml::from_str::<Spec>(&printed).unwrap();
    assert!(reparsed.collections == spec.collections);
    assert_eq!(
        reparsed
            .collections
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["zebra", "books"]
    );
    assert_eq!(
        reparsed.collections["books"]
            .fields
            .keys()
            .collect::<Vec<_>>(),
        ["title", "embedding", "page_counts"]
    );
    assert_eq!(toml::to_string_pretty(&reparsed).unwrap(), printed);
}

#[rstest]
#[case::exact_text(FieldSpec::text(true).with_index(FieldIndex::keyword(KeywordIndexType::Exact)))]
#[case::dense_binary(FieldSpec::binary_vector(16, false).with_index(FieldIndex::vector(VectorDistanceMetric::Hamming)))]
#[case::sparse_f16(FieldSpec::f16_sparse_vector(false).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)))]
#[case::matrix_u8(FieldSpec::matrix(true, 3, MatrixValueType::U8).with_index(FieldIndex::multi_vector(MultiVectorDistanceMetric::Maxsim, Some(MultiVectorQuantization::Binary2bit), None, None).skip_smve()))]
#[case::nested_schema(FieldSpec::r#struct(true, [("child", FieldSpec::integer(true)), ("nested", FieldSpec::r#struct(false, [("text", FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Exact)))]))]))]
#[case::vector_exact(FieldSpec::f32_vector(3, false).with_index(FieldIndex { index: Some(field_index::Index::VectorIndex(VectorIndex { metric: VectorDistanceMetric::Cosine as i32, exact: Some(true) })) }))]
#[case::matrix_metadata(FieldSpec::matrix(false, 3, MatrixValueType::F32).with_index(FieldIndex { index: Some(field_index::Index::MultiVectorIndex(MultiVectorIndex { metric: MultiVectorDistanceMetric::Maxsim as i32, quantization: Some(MultiVectorQuantization::Scalar as i32), width: Some(4096), top_k: Some(16), encoding_version: 2, ..Default::default() })) }))]
#[case::semantic_metadata(FieldSpec::text(false).with_index(FieldIndex { index: Some(field_index::Index::SemanticIndex(SemanticIndex { model: Some("preserved-model".into()), embedding_type: Some(1) })) }))]
#[allow(deprecated)]
fn schema_conversion_round_trip(#[case] spec: FieldSpec) {
    let field = Field::new(spec.clone());
    let encoded = toml::to_string(&field).unwrap();
    let parsed: Field = toml::from_str(&encoded).unwrap();
    assert!(field == parsed);
}

#[rstest]
#[case::missing_type(FieldSpec::default())]
#[case::scalar_index(FieldSpec::integer(false).with_index(FieldIndex::semantic()))]
#[case::vector_text_index(FieldSpec::f32_vector(3, false).with_index(FieldIndex::semantic()))]
#[case::image_unsupported(FieldSpec::image(false))]
#[case::invalid_nested(FieldSpec::r#struct(false, [("child", FieldSpec::default())]))]
#[case::missing_inner_type(FieldSpec { data_type: Some(FieldType::default()), ..Default::default() })]
#[case::unknown_matrix_element(FieldSpec::matrix(false, 3, MatrixValueType::Unspecified))]
fn invalid_source_fields(#[case] spec: FieldSpec) {
    assert!(Field::new(spec).validate().is_err());
}

#[rstest]
#[case::unknown_key("text", "bogus = 1", "unknown field `bogus`")]
#[case::unknown_type("quaternion", "", "quaternion")]
#[case::unknown_index("text", "index = 'fuzzy'", "fuzzy")]
#[case::vector_without_dim("f32_vector", "", "missing field `dim`")]
#[case::matrix_without_cols("f32_matrix", "", "missing field `cols`")]
#[case::scalar_dimension("text", "dim = 3", "unknown field `dim`")]
#[case::sparse_dimension("f32_sparse_vector", "dim = 3", "unknown field `dim`")]
#[case::vector_columns("f32_vector", "dim = 3\ncols = 3", "unknown field `cols`")]
#[case::truncate_on_int("int", "truncate = 5", "unknown field `truncate`")]
#[case::semantic_rejects_bytes("bytes", "index = 'semantic'", "incompatible index")]
#[case::keyword_needs_text("int", "index = 'keyword'", "incompatible index")]
#[case::semantic_needs_text("int", "index = 'semantic'", "incompatible index")]
#[case::vector_needs_vector(
    "text",
    "index = { vector = { metric = 'cosine' } }",
    "incompatible index"
)]
#[case::multi_vector_needs_matrix("text", "index = { multi_vector = {} }", "incompatible index")]
fn field_validation(#[case] ty: &str, #[case] options: &str, #[case] fragment: &str) {
    let message = refused(toml::from_str::<Field>(&format!(
        "type = {ty:?}\n{options}"
    )));
    assert!(message.contains(fragment), "got: {message}");
}

#[test]
fn zero_values_parse() {
    let spec: Spec = toml::from_str("[c]\nfrom = 'f.parquet'\nlimit = 0\n[c.fields]\nt = { type = 'text', truncate = 0 }\nv = { type = 'f32_vector', dim = 0 }\nm = { type = 'f32_matrix', cols = 0 }").unwrap();
    assert_eq!(spec.collections["c"].limit, Some(0));
    assert_eq!(spec.collections["c"].fields["t"].truncate, Some(0));
    assert_eq!(
        spec.collections["c"].fields["v"].schema,
        FieldSpec::f32_vector(0, false)
    );
    assert_eq!(
        spec.collections["c"].fields["m"].schema,
        FieldSpec::matrix(false, 0, MatrixValueType::F32)
    );
}

#[rstest]
#[case::binary_matrix("binary_matrix", "cols = 8")]
#[case::binary_sparse("binary_sparse_vector", "")]
fn unsupported_shapes(#[case] name: &str, #[case] options: &str) {
    assert!(toml::from_str::<Field>(&format!("type = {name:?}\n{options}")).is_err());
}

#[test]
fn element_is_not_a_wire_key() {
    assert!(toml::from_str::<Field>("type = 'f32_vector'\ndim = 3\nelement = 'f16'").is_err());
}

#[rstest]
#[case::text("text", "", FieldSpec::text(false))]
#[case::f32_vector("f32_vector", "dim = 3", FieldSpec::f32_vector(3, false))]
#[case::f16_vector("f16_vector", "dim = 3", FieldSpec::f16_vector(3, false))]
#[case::f8_vector("f8_vector", "dim = 3", FieldSpec::f8_vector(3, false))]
#[case::u8_vector("u8_vector", "dim = 3", FieldSpec::u8_vector(3, false))]
#[case::i8_vector("i8_vector", "dim = 3", FieldSpec::i8_vector(3, false))]
#[case::f32_sparse_vector("f32_sparse_vector", "", FieldSpec::f32_sparse_vector(false))]
#[case::f16_sparse_vector("f16_sparse_vector", "", FieldSpec::f16_sparse_vector(false))]
#[case::f8_sparse_vector("f8_sparse_vector", "", FieldSpec::f8_sparse_vector(false))]
#[case::u8_sparse_vector("u8_sparse_vector", "", FieldSpec::u8_sparse_vector(false))]
#[case::i8_sparse_vector("i8_sparse_vector", "", FieldSpec::i8_sparse_vector(false))]
#[case::f32_matrix(
    "f32_matrix",
    "cols = 3",
    FieldSpec::matrix(false, 3, MatrixValueType::F32)
)]
#[case::f16_matrix(
    "f16_matrix",
    "cols = 3",
    FieldSpec::matrix(false, 3, MatrixValueType::F16)
)]
#[case::f8_matrix(
    "f8_matrix",
    "cols = 3",
    FieldSpec::matrix(false, 3, MatrixValueType::F8)
)]
#[case::u8_matrix(
    "u8_matrix",
    "cols = 3",
    FieldSpec::matrix(false, 3, MatrixValueType::U8)
)]
#[case::i8_matrix(
    "i8_matrix",
    "cols = 3",
    FieldSpec::matrix(false, 3, MatrixValueType::I8)
)]
#[case::binary_vector("binary_vector", "dim = 3", FieldSpec::binary_vector(3, false))]
fn field_wire_shapes(#[case] name: &str, #[case] options: &str, #[case] schema: FieldSpec) {
    let field: Field = toml::from_str(&format!("type = {name:?}\n{options}")).unwrap();
    assert_eq!(field.schema, schema);
    let encoded = toml::to_string(&Field::new(schema)).unwrap();
    let printed: toml::Value = toml::from_str(&encoded).unwrap();
    assert_eq!(printed["type"].as_str(), Some(name));
    assert!(printed.get("element").is_none());
    assert!(toml::from_str::<Field>(&encoded).unwrap() == field);
    let json = serde_json::to_value(&field).unwrap();
    assert_eq!(json["type"], name);
    assert!(serde_json::from_value::<Field>(json).unwrap() == field);
}

#[rstest]
#[case::nan_truncate("truncate = nan")]
#[case::infinite_truncate("truncate = inf")]
#[case::nan_index("index = nan")]
#[case::nan_source("from = nan")]
fn nonfinite_options_are_rejected(#[case] option: &str) {
    assert!(toml::from_str::<Field>(&format!("type = 'text'\n{option}")).is_err());
}

#[rstest]
#[case::nested_rename("type = 'struct'\nfields = { child = { type = 'text', from = 'other' } }")]
#[case::nested_truncate("type = 'struct'\nfields = { child = { type = 'text', truncate = 5 } }")]
#[case::semantic_unknown("type = 'text'\nindex = { semantic = { bogus = 1 } }")]
#[case::vector_unknown(
    "type = 'f32_vector'\ndim = 3\nindex = { vector = { metric = 'cosine', bogus = 1 } }"
)]
fn unsupported_options(#[case] input: &str) {
    assert!(toml::from_str::<Field>(input).is_err());
}

#[rstest]
#[case::missing_schema(Field::new(FieldSpec::default()), "has no data type")]
#[case::scalar_truncate(Field { truncate: Some(5), ..Field::new(FieldSpec::integer(false)) }, "unknown field `truncate`")]
#[tokio::test]
async fn documents_validate_target(#[case] field: Field, #[case] fragment: &str) {
    let source = Uri::Files(None).connect().await.unwrap();
    let target = Target {
        from: "unused.parquet".into(),
        limit: Some(0),
        fields: [("value".into(), field)].into(),
        id: "_id".into(),
        filter: None,
        partition: None,
    };
    assert!(source.scan(&target, None).await.is_ok());
    let error = refused(source.documents(&target).await);
    assert!(error.contains(fragment), "{error}");
    assert!(Spec::try_from(indexmap::IndexMap::from([("c".into(), target)])).is_err());
}

#[rstest]
#[case::empty_id("")]
#[case::blank_id(" ")]
fn empty_id_rejected(#[case] id: &str) {
    let input =
        format!("[c]\nfrom = 'rows.parquet'\nid = {id:?}\n[c.fields]\ntitle = {{ type = 'text' }}");
    assert!(refused(toml::from_str::<Spec>(&input)).contains("`id` cannot be empty"));
}
