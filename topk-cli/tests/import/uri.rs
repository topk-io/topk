use rstest::rstest;
use topk::import::source::{File, Uri};
use topk::import::source::{Format, ObjectStore};

#[rstest]
#[case::postgres("postgres://u:p@localhost/db", "postgres://u:p@localhost/db")]
#[case::postgresql("postgresql://localhost/db", "postgresql://localhost/db")]
#[case::bare_postgres("postgres://", "postgres://")]
#[case::surrounding_space("  postgres://localhost/db  ", "postgres://localhost/db")]
fn postgres_urls(#[case] input: &str, #[case] expected: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Postgres(url) => assert_eq!(url.as_str(), expected),
        uri => panic!("expected PostgreSQL, got {uri:?}"),
    }
}

#[rstest]
#[case::mysql("mysql://root@localhost:3307/demo")]
#[case::mariadb("mariadb://localhost/demo")]
fn mysql_urls(#[case] input: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Mysql(url) => assert_eq!(url.as_str(), input),
        uri => panic!("expected MySQL, got {uri:?}"),
    }
}

#[rstest]
#[case::mongo("mongodb://localhost:27017/demo")]
#[case::mongo_srv("mongodb+srv://cluster.example.com/demo")]
fn mongo_urls(#[case] input: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Mongo(url) => assert_eq!(url.as_str(), input),
        uri => panic!("expected MongoDB, got {uri:?}"),
    }
}

#[rstest]
#[case::long_http("elasticsearch://localhost:19200", "http://localhost:19200/")]
#[case::explicit_http("elasticsearch+http://localhost:19200", "http://localhost:19200/")]
#[case::long_https("elasticsearch+https://es.example.com", "https://es.example.com/")]
#[case::short_http("es://localhost:19200", "http://localhost:19200/")]
#[case::short_https("es+https://es.example.com", "https://es.example.com/")]
#[case::short_explicit_http("es+http://localhost:19200", "http://localhost:19200/")]
#[case::elastic_cloud(
    "https://demo.es.us-east-1.aws.cloud.es.io",
    "https://demo.es.us-east-1.aws.cloud.es.io/"
)]
#[case::elastic_serverless(
    "https://demo.es.us-east-1.aws.elastic.cloud",
    "https://demo.es.us-east-1.aws.elastic.cloud/"
)]
fn elasticsearch_urls(#[case] input: &str, #[case] expected: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Elasticsearch(url) => assert_eq!(url.as_str(), expected),
        uri => panic!("expected Elasticsearch, got {uri:?}"),
    }
}

#[rstest]
#[case::absolute_path("sqlite:///tmp/books.db", std::path::PathBuf::from("/tmp/books.db"))]
#[case::relative_path("sqlite:books.db", std::env::current_dir().unwrap().join("books.db"))]
#[case::home_path("sqlite:~/books.db", dirs::home_dir().unwrap().join("books.db"))]
fn sqlite_paths(#[case] input: &str, #[case] expected: std::path::PathBuf) {
    match input.parse::<Uri>().unwrap() {
        Uri::Sqlite(path) => assert_eq!(std::path::Path::new(&path), expected),
        uri => panic!("expected SQLite, got {uri:?}"),
    }
}

#[rstest]
#[case::local_file("./books.parquet")]
#[case::http_file("https://example.com/books.parquet")]
#[case::bucket_file("s3://bucket/books.parquet")]
fn routes_to_files(#[case] input: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Files(Some(file)) => assert_eq!(file.path, input),
        uri => panic!("expected a file, got {uri:?}"),
    }
}

#[rstest]
#[case::collection_only("topk://books", None)]
#[case::explicit_project("topk://source/books", Some("source"))]
fn topk_targets(#[case] input: &str, #[case] project: Option<&str>) {
    match input.parse::<Uri>().unwrap() {
        Uri::Topk {
            collection,
            project: actual,
        } => {
            assert_eq!(collection, "books");
            assert_eq!(actual.as_ref().map(|id| id.as_str()), project);
        }
        uri => panic!("expected TopK, got {uri:?}"),
    }
}

#[rstest]
#[case::http_file(
    "http://example.com/data/books.parquet",
    Some(ObjectStore::Http),
    Format::Parquet
)]
#[case::https_presigned(
    "https://example.com/books.csv?X-Amz-Signature=abc",
    Some(ObjectStore::Http),
    Format::Csv
)]
#[case::s3("s3://bucket/books.parquet", Some(ObjectStore::S3), Format::Parquet)]
#[case::r2("r2://bucket/books.parquet", Some(ObjectStore::S3), Format::Parquet)]
#[case::gs("gs://bucket/books.parquet", Some(ObjectStore::Gcs), Format::Parquet)]
#[case::gcs("gcs://bucket/books.parquet", Some(ObjectStore::Gcs), Format::Parquet)]
#[case::azure_short(
    "az://container/books.parquet",
    Some(ObjectStore::Azure),
    Format::Parquet
)]
#[case::azure(
    "azure://container/books.parquet",
    Some(ObjectStore::Azure),
    Format::Parquet
)]
#[case::huggingface(
    "hf://datasets/org/name/default/train-00000-of-00001.parquet",
    Some(ObjectStore::HuggingFace),
    Format::Parquet
)]
#[case::huggingface_converted(
    "hf://datasets/org/name@~parquet/default/train/*.parquet",
    Some(ObjectStore::HuggingFace),
    Format::Parquet
)]
#[case::relative_file("./books.parquet", None, Format::Parquet)]
#[case::absolute_glob("/data/part_*.csv", None, Format::Csv)]
#[case::avro_file("books.avro", None, Format::Avro)]
#[case::bucket_tilde("s3://bucket/~/books.parquet", Some(ObjectStore::S3), Format::Parquet)]
#[case::http_tilde(
    "https://example.com/~books/data.csv",
    Some(ObjectStore::Http),
    Format::Csv
)]
fn files(#[case] input: &str, #[case] store: Option<ObjectStore>, #[case] format: Format) {
    assert_eq!(
        input.parse::<File>().unwrap(),
        File {
            path: input.into(),
            store,
            format
        }
    );
}

#[rstest]
#[case::home_file("data/books.parquet")]
#[case::home_glob("data/*.parquet")]
fn file_home_expands(#[case] path: &str) {
    let file = format!("~/{path}").parse::<File>().unwrap();
    assert_eq!(
        file,
        File {
            path: dirs::home_dir().unwrap().join(path).display().to_string(),
            store: None,
            format: Format::Parquet,
        }
    );
}

#[rstest]
#[case::empty("", "empty source uri")]
#[case::only_space("   ", "empty source uri")]
#[case::mongo_without_a_database("mongodb://localhost:27017", "must include a database")]
#[case::malformed("postgres://[oops", "bad source uri")]
#[case::former_api_key("topk://key@dev/books", "does not take an API key")]
#[case::empty_source_project("topk:///books", "invalid project ID")]
#[case::old_query("topk://books?project_id=source", "exact collection name")]
#[case::extra_segment("topk://source/books/other", "cannot contain")]
#[case::all_collections("topk://", "exact collection name")]
#[case::collection_glob("topk://books*", "exact collection name")]
fn bad_uris(#[case] input: &str, #[case] fragment: &str) {
    let message = match input.parse::<Uri>() {
        Err(error) => error.to_string(),
        Ok(uri) => panic!("expected a refusal, got {uri:?}"),
    };
    assert!(message.contains(fragment), "got: {message}");
}

#[test]
fn malformed_uri_redacts_password() {
    let error = "postgres://user:hunter2@[oops".parse::<Uri>().unwrap_err();
    let error = anyhow::Error::new(error);
    let cause = error.chain().last().unwrap().to_string();
    let message = format!("{error:#}");
    assert!(message.contains("bad source uri"), "{message}");
    assert!(!message.contains("hunter2"), "{message}");
    assert_eq!(message.matches(&cause).count(), 1, "{message}");
}

#[rstest]
#[case::unknown_file_type("data.txt", "cannot tell the file type")]
#[case::no_extension("data/books", "cannot tell the file type")]
#[case::huggingface_dataset("hf://datasets/org/name", "@~parquet")]
fn bad_files(#[case] input: &str, #[case] fragment: &str) {
    let message = input.parse::<File>().unwrap_err().to_string();
    assert!(message.contains(fragment), "got: {message}");
}
