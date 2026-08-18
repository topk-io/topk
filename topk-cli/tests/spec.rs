use rstest::rstest;
use topk::import::{Error, Spec, Uri};

fn parse_error(toml: &str) -> Error {
    match Spec::parse(toml) {
        Err(error) => error,
        Ok(_) => panic!("expected the spec to be rejected:\n{toml}"),
    }
}

#[rstest]
#[case::postgres("postgres://u:p@localhost/db", "Postgres")]
#[case::postgresql("postgresql://localhost/db", "Postgres")]
#[case::bare_postgres("postgres://", "Postgres")]
#[case::mysql("mysql://root@localhost:3307/demo", "Mysql")]
#[case::mariadb("mariadb://localhost/demo", "Mysql")]
#[case::sqlite_slashes("sqlite:///tmp/books.db", "Sqlite")]
#[case::sqlite_colon("sqlite:books.db", "Sqlite")]
#[case::mongo("mongodb://localhost:27017/demo", "Mongo")]
#[case::mongo_srv("mongodb+srv://cluster.example.com/demo", "Mongo")]
#[case::elasticsearch("elasticsearch://localhost:19200", "Elasticsearch")]
#[case::elasticsearch_https("elasticsearch+https://es.example.com", "Elasticsearch")]
#[case::es_short("es://localhost:19200", "Elasticsearch")]
#[case::es_short_https("es+https://es.example.com", "Elasticsearch")]
#[case::elastic_cloud("https://demo.es.us-east-1.aws.cloud.es.io", "Elasticsearch")]
#[case::elastic_cloud_serverless("https://demo.es.us-east-1.aws.elastic.cloud", "Elasticsearch")]
#[case::http_file("http://example.com/data/books.parquet", "Http")]
#[case::https_presigned("https://example.com/books.csv?X-Amz-Signature=abc", "Csv")]
#[case::s3("s3://bucket/books.parquet", "S3")]
#[case::r2("r2://bucket/books.parquet", "S3")]
#[case::gs("gs://bucket/books.parquet", "Gcs")]
#[case::gcs("gcs://bucket/books.parquet", "Gcs")]
#[case::azure_short("az://container/books.parquet", "Azure")]
#[case::azure("azure://container/books.parquet", "Azure")]
#[case::huggingface(
    "hf://datasets/org/name/default/train-00000-of-00001.parquet",
    "HuggingFace"
)]
#[case::huggingface_converted(
    "hf://datasets/org/name@~parquet/default/train/*.parquet",
    "HuggingFace"
)]
#[case::relative_file("./books.parquet", "store: None")]
#[case::absolute_glob("/data/part_*.csv", "store: None")]
#[case::avro("books.avro", "Avro")]
#[case::xlsx("sheet.xlsx", "Xlsx")]
#[case::surrounding_space("  postgres://localhost/db  ", "Postgres")]
fn uris(#[case] input: &str, #[case] expected: &str) {
    let uri: Uri = input.parse().expect("uri parses");
    assert!(format!("{uri:?}").contains(expected), "{uri:?}");
}

#[rstest]
#[case::file("~/data/books.parquet")]
#[case::glob("~/data/*.parquet")]
#[case::sqlite("sqlite:~/books.db")]
fn tilde_expands(#[case] input: &str) {
    let uri: Uri = input.parse().expect("uri parses");
    assert!(!format!("{uri:?}").contains('~'), "{uri:?}");
}

#[rstest]
#[case::s3("s3://bucket/~/books.parquet")]
#[case::http("https://example.com/~books/data.csv")]
fn tilde_survives_remote_paths(#[case] input: &str) {
    let uri: Uri = input.parse().expect("uri parses");
    assert!(format!("{uri:?}").contains('~'), "{uri:?}");
}

#[rstest]
#[case::empty("", "empty source uri")]
#[case::only_space("   ", "empty source uri")]
#[case::mongo_without_a_database("mongodb://localhost:27017", "must include a database")]
#[case::malformed("postgres://[oops", "bad source uri")]
#[case::unknown_file_type("data.txt", "cannot tell the file type")]
#[case::no_extension("data/books", "cannot tell the file type")]
#[case::huggingface_dataset("hf://datasets/org/name", "@~parquet")]
fn bad_uris(#[case] input: &str, #[case] fragment: &str) {
    let message = match input.parse::<Uri>() {
        Err(Error::InvalidArgument(message)) => message,
        Err(other) => panic!("expected InvalidArgument, got {other:?}"),
        Ok(uri) => panic!("expected a refusal, got {uri:?}"),
    };
    assert!(message.contains(fragment), "got: {message}");
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
    let message = match parse_error(&toml) {
        Error::InvalidArgument(message) => message,
        other => panic!("expected InvalidArgument, got {other:?}"),
    };
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
    let spec = Spec::parse(&toml).expect("spec parses");
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
fn invalid_targets(#[case] toml: &str, #[case] fragment: &str) {
    let message = match parse_error(toml) {
        Error::InvalidArgument(message) => message,
        other => panic!("expected InvalidArgument, got {other:?}"),
    };
    assert!(message.contains(fragment), "got: {message}");
}

#[rstest]
#[case::missing_from(
    r#"
[c]
id = "_id"
"#
)]
#[case::unknown_target_key(
    r#"
[c]
from = "f.parquet"
bogus = 1
"#
)]
#[case::unknown_field_key(
    r#"
[c]
from = "f.parquet"

[c.fields]
v = { type = "text", bogus = 1 }
"#
)]
#[case::unknown_type(
    r#"
[c]
from = "f.parquet"

[c.fields]
v = { type = "quaternion" }
"#
)]
#[case::unknown_index(
    r#"
[c]
from = "f.parquet"

[c.fields]
v = { type = "text", index = "fuzzy" }
"#
)]
fn unknown_keys(#[case] toml: &str) {
    match parse_error(toml) {
        Error::Toml(_) => {}
        other => panic!("expected a TOML error, got {other:?}"),
    }
}

#[test]
fn minimal_target() {
    let spec = Spec::parse(
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
    let message = match parse_error("[c]\nfrom = \"books.parquet\"\n") {
        Error::InvalidArgument(message) => message,
        other => panic!("expected InvalidArgument, got {other:?}"),
    };
    assert!(
        message.contains("declare at least one field"),
        "got: {message}"
    );
}

#[test]
fn selection_options() {
    let spec = Spec::parse(
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
    let spec = Spec::parse(
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
    let printed = toml::to_string_pretty(&Spec::parse(toml).expect("spec parses")).expect("prints");
    let reparsed = Spec::parse(&printed).expect("printed spec reads back");
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
