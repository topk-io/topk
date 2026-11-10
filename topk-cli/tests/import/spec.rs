use crate::import::common::{refused, spec_toml};
use rstest::rstest;
use topk::import::{Field, Spec};
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::control::{
    FieldIndex, FieldSpec, KeywordIndexType, MultiVectorDistanceMetric, MultiVectorQuantization,
    VectorDistanceMetric,
};

fn parse_error(toml: &str) -> String {
    match toml::from_str::<Spec>(toml) {
        Err(error) => error.to_string(),
        Ok(_) => panic!("expected the spec to be rejected:\n{toml}"),
    }
}

#[rstest]
#[case::leading_underscore("_books", "collection names start with a letter or digit")]
#[case::leading_hyphen("-books", "collection names start with a letter or digit")]
#[case::leading_dot(".books", "collection names start with a letter or digit")]
#[case::inner_space("my books", "collection names start with a letter or digit")]
fn invalid_collection_names(#[case] name: &str, #[case] fragment: &str) {
    let toml = format!(
        "[{name:?}]\nfrom = \"f.parquet\"\n\n[{name:?}.fields]\nt = {{ type = \"text\" }}\n"
    );
    let message = parse_error(&toml);
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::mixed_case("myBooks")]
#[case::leading_digit("2026-logs")]
#[case::dotted("logs-2026.07.29")]
#[case::digits_and_separators("b9_books-v2")]
fn valid_collection_names(#[case] name: &str) {
    let toml = format!(
        "[{name:?}]\nfrom = \"f.parquet\"\n\n[{name:?}.fields]\nt = {{ type = \"text\" }}\n"
    );
    let spec = toml::from_str::<Spec>(&toml).expect("spec parses");
    assert!(spec.collections.contains_key(name), "{name}");
}

#[rstest]
#[case::empty_from(
    r#"
[c]
from = ""

[c.fields]
t = { type = "text" }
"#,
    "`from` is empty"
)]
#[case::blank_from(
    r#"
[c]
from = "   "

[c.fields]
t = { type = "text" }
"#,
    "`from` is empty"
)]
#[case::truncate_on_a_non_text_field(
    r#"
[c]
from = "f.parquet"

[c.fields]
n = { type = "int", truncate = 5 }
"#,
    "does not take `truncate`"
)]
#[case::truncate_to_zero(
    r#"
[c]
from = "f.parquet"

[c.fields]
t = { type = "text", truncate = 0 }
"#,
    "at least 1 character"
)]
#[case::a_field_named_underscore_id(
    r#"
[c]
from = "f.parquet"

[c.fields]
_id = { type = "text" }
"#,
    "cannot be empty or start with `_`"
)]
#[case::a_field_with_reserved_prefix(
    r#"
[c]
from = "f.parquet"

[c.fields]
_score = { type = "float" }
"#,
    "cannot be empty or start with `_`"
)]
#[case::zero_limit(
    "[c]\nfrom = 'f.parquet'\nlimit = 0\n[c.fields]\nt = { type = 'text' }",
    "limit must be greater than zero"
)]
#[case::zero_vector_dimension(
    "[c]\nfrom = 'f.parquet'\n[c.fields]\nv = { type = 'f32_vector', dim = 0 }",
    "`dim` must be greater than zero"
)]
#[case::zero_matrix_columns(
    "[c]\nfrom = 'f.parquet'\n[c.fields]\nv = { type = 'f32_matrix', cols = 0 }",
    "`cols` must be greater than zero"
)]
fn invalid_targets(#[case] toml: &str, #[case] fragment: &str) {
    let message = parse_error(toml);
    assert!(message.contains(fragment), "got: {message}");
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
#[case::unknown_field_key(
    "[c]\nfrom = 'f.parquet'\n[c.fields]\nv = { type = 'text', bogus = 1 }",
    "unknown field `bogus`"
)]
#[case::unknown_type(
    "[c]\nfrom = 'f.parquet'\n[c.fields]\nv = { type = 'quaternion' }",
    "quaternion"
)]
#[case::unknown_index(
    "[c]\nfrom = 'f.parquet'\n[c.fields]\nv = { type = 'text', index = 'fuzzy' }",
    "fuzzy"
)]
fn unknown_keys(#[case] toml: &str, #[case] fragment: &str) {
    let message = parse_error(toml);
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
    assert_eq!(target.id, None);
    assert_eq!(target.fields.len(), 1);
}

#[test]
fn no_fields_is_rejected() {
    let message = parse_error("[c]\nfrom = \"books.parquet\"\n");
    assert!(
        message.contains("declare at least one field"),
        "got: {message}"
    );
}

#[test]
fn selection_options() {
    let spec = toml::from_str::<Spec>(
        r#"
[c]
from = "public.books"
id = "sku"
filter = "published_year > 1950"
limit = 25

[c.fields]
t = { type = "text" }
"#,
    )
    .expect("spec parses");
    let target = &spec.collections["c"];
    assert_eq!(target.filter.as_deref(), Some("published_year > 1950"));
    assert_eq!(target.limit, Some(25));
}

#[test]
fn declared_order() {
    let spec = toml::from_str::<Spec>(
        r#"
[zebra]
from = "z.parquet"

[zebra.fields]
t = { type = "text" }

[apple]
from = "a.parquet"

[apple.fields]
t = { type = "text" }

[mango]
from = "m.parquet"

[mango.fields]
t = { type = "text" }
"#,
    )
    .expect("spec parses");
    assert_eq!(
        spec.collections
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["zebra", "apple", "mango"]
    );
}

#[test]
fn serialize_round_trip() {
    let toml = r#"
[books]
from = "public.books"
id = "sku"
limit = 10

[books.fields]
title = { type = "text", required = true, index = "keyword" }
embedding = { from = "vec", type = "f32_vector", dim = 3, index = { vector = { metric = "cosine" } } }
page_counts = { type = "int_list" }
"#;
    let printed = toml::to_string_pretty(&toml::from_str::<Spec>(toml).expect("spec parses"))
        .expect("prints");
    let reparsed = toml::from_str::<Spec>(&printed).expect("printed spec reads back");
    assert_eq!(reparsed.collections["books"].from, "public.books");
    assert_eq!(reparsed.collections["books"].id.as_deref(), Some("sku"));
    assert!(reparsed.collections["books"].fields["title"].required);
    assert_eq!(reparsed.collections["books"].limit, Some(10));
    assert_eq!(
        reparsed.collections["books"]
            .fields
            .keys()
            .collect::<Vec<_>>(),
        ["title", "embedding", "page_counts"]
    );
    assert_eq!(
        toml::to_string_pretty(&reparsed).expect("prints"),
        printed,
        "printing is not idempotent"
    );
}

#[rstest]
#[case::exact_text(FieldSpec::text(true).with_index(FieldIndex::keyword(KeywordIndexType::Exact)))]
#[case::image(FieldSpec::image(false))]
#[case::dense_binary(FieldSpec::binary_vector(16, false).with_index(FieldIndex::vector(VectorDistanceMetric::Hamming)))]
#[case::sparse_f16(FieldSpec::f16_sparse_vector(false).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)))]
#[case::matrix_u8(FieldSpec::matrix(true, 3, MatrixValueType::U8).with_index(FieldIndex::multi_vector(MultiVectorDistanceMetric::Maxsim, Some(MultiVectorQuantization::Binary2bit), None, None).skip_smve()))]
fn schema_conversion_round_trip(#[case] spec: FieldSpec) {
    let field = Field::try_from(&spec).unwrap();
    assert_eq!(FieldSpec::try_from(&field).unwrap(), spec);
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
    "a semantic index needs a `text` or `image` field"
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
