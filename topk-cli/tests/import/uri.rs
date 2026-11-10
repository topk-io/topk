use rstest::rstest;
use topk::import::Uri;

#[test]
fn sqlite_source_is_bound_to_the_original_directory() {
    let parsed: Uri = "sqlite:books.db".parse().unwrap();
    let expected = std::env::current_dir().unwrap().join("books.db");

    assert_eq!(parsed.to_string(), format!("sqlite:{}", expected.display()));
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
#[case::elasticsearch("elasticsearch://localhost:19200", "Es")]
#[case::elasticsearch_https("elasticsearch+https://es.example.com", "Es")]
#[case::es_short("es://localhost:19200", "Es")]
#[case::es_short_https("es+https://es.example.com", "Es")]
#[case::es_short_http("es+http://localhost:19200", "Es")]
#[case::elastic_cloud("https://demo.es.us-east-1.aws.cloud.es.io", "Es")]
#[case::elastic_cloud_serverless("https://demo.es.us-east-1.aws.elastic.cloud", "Es")]
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
