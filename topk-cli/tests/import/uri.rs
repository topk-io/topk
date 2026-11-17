use rstest::rstest;
use topk::import::source::{File, SourceLocation, Uri};
use topk::import::source::{Format, ObjectStore};

#[rstest]
#[case::postgres(
    "postgres://u:p@localhost/db?sslmode=require#public.orders",
    "public.orders"
)]
#[case::postgresql("postgresql://localhost/db#orders", "orders")]
#[case::bare_postgres("postgres://#orders", "orders")]
#[case::mysql("mysql://root@localhost:3307/demo#orders", "orders")]
#[case::mariadb("mariadb://localhost/demo#orders", "orders")]
#[case::mongo("mongodb://localhost:27017/demo#orders", "orders")]
#[case::mongo_srv("mongodb+srv://cluster.example.com/demo#orders", "orders")]
#[case::encoded_name("postgres://localhost/db#orders%20%23%25%C3%A9", "orders #%é")]
fn database_urls(#[case] input: &str, #[case] expected: &str) {
    let uri: Uri = input.parse().unwrap();
    match &uri {
        Uri::Postgres { url, object } | Uri::Mysql { url, object } | Uri::Mongo { url, object } => {
            assert_eq!(object, expected);
            assert_eq!(url.0.as_str(), input.split_once('#').unwrap().0);
            assert_eq!(url.0.fragment(), None);
        }
        uri => panic!("expected database, got {uri:?}"),
    }
    assert_eq!(
        uri.to_string().parse::<Uri>().unwrap().to_string(),
        uri.to_string()
    );
}

#[rstest]
#[case::long_http(
    "elasticsearch://localhost:19200/orders",
    "http://localhost:19200/orders"
)]
#[case::explicit_http(
    "elasticsearch+http://localhost:19200/orders",
    "http://localhost:19200/orders"
)]
#[case::long_https(
    "elasticsearch+https://es.example.com/orders",
    "https://es.example.com/orders"
)]
#[case::short_http("es://localhost:19200/orders", "http://localhost:19200/orders")]
#[case::short_https("es+https://es.example.com/orders", "https://es.example.com/orders")]
#[case::elastic_cloud(
    "https://demo.es.us-east-1.aws.cloud.es.io/orders",
    "https://demo.es.us-east-1.aws.cloud.es.io/orders"
)]
fn elasticsearch_urls(#[case] input: &str, #[case] expected: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Elasticsearch { url, index } => {
            assert_eq!(index, "orders");
            assert_eq!(url.0.as_str(), expected.strip_suffix("orders").unwrap());
        }
        uri => panic!("expected Elasticsearch, got {uri:?}"),
    }
}

#[rstest]
#[case::postgres("postgres://u:hunter2@localhost/db#orders")]
#[case::mysql("mysql://u:hunter2@localhost/db#orders")]
#[case::mongo("mongodb://u:hunter2@localhost/db#orders")]
#[case::elasticsearch("es+https://u:hunter2@es.example.com/orders")]
fn printing_hides_the_password(#[case] input: &str) {
    let uri = input.parse::<Uri>().unwrap();
    for printed in [uri.to_string(), format!("{uri:?}")] {
        assert!(
            printed.contains("u:***@") && !printed.contains("hunter2"),
            "{printed}"
        );
    }
}

#[rstest]
#[case::absolute_path(
    "sqlite:///tmp/books.db#books",
    std::path::PathBuf::from("/tmp/books.db")
)]
#[case::relative_path("sqlite:books.db#books", std::env::current_dir().unwrap().join("books.db"))]
#[case::home_path("sqlite:~/books.db#books", dirs::home_dir().unwrap().join("books.db"))]
#[case::encoded_path("sqlite:books%23%25.db#books", std::env::current_dir().unwrap().join("books#%.db"))]
fn sqlite_paths(#[case] input: &str, #[case] expected: std::path::PathBuf) {
    let uri: Uri = input.parse().unwrap();
    match &uri {
        Uri::Sqlite { path, object } => {
            assert_eq!(std::path::Path::new(path), expected);
            assert_eq!(object, "books");
        }
        uri => panic!("expected SQLite, got {uri:?}"),
    }
    assert_eq!(
        uri.to_string().parse::<Uri>().unwrap().to_string(),
        uri.to_string()
    );
}

#[rstest]
#[case::local_fragment("./books#edition.parquet")]
#[case::http_fragment("https://example.com/books.parquet?token=x#part")]
#[case::huggingface_file("hf://datasets/org/name@~parquet/default/train/*.parquet")]
#[case::local_file("./books.parquet")]
#[case::http_file("https://example.com/books.parquet")]
#[case::bucket_file("s3://bucket/books.parquet")]
fn routes_to_files(#[case] input: &str) {
    match input.parse::<Uri>().unwrap() {
        Uri::Files(file) => assert_eq!(
            file.path,
            match file.store {
                None => std::path::absolute(input).unwrap().display().to_string(),
                Some(_) => input.to_string(),
            }
        ),
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
            path: match store {
                None => std::path::absolute(input).unwrap().display().to_string(),
                Some(_) => input.into(),
            },
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
#[case::missing_index("es://localhost", "requires one index")]
#[case::multiple_indices("es://localhost/a,b", "requires one index")]
#[case::encoded_glob("es://localhost/a%2A", "requires one index")]
#[case::missing_selector(
    "postgres://localhost/db",
    "postgres source requires #<table>, e.g. postgres://host/db#orders"
)]
#[case::empty_selector(
    "mysql://localhost/db#",
    "mysql source requires #<table>, e.g. mysql://host/db#orders"
)]
#[case::sqlite_selector(
    "sqlite:books.db",
    "sqlite source requires #<table>, e.g. sqlite:books.db#orders"
)]
#[case::mongo_selector(
    "mongodb://localhost/shop",
    "mongodb source requires #<collection>, e.g. mongodb://host/db#products"
)]
#[case::bad_escape("postgres://localhost/db#bad%2", "invalid percent encoding")]
#[case::bad_hex("postgres://localhost/db#bad%zz", "invalid percent encoding")]
#[case::invalid_utf8("mongodb://localhost/db#bad%FF", "not UTF-8")]
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
#[case::elasticsearch_index("http://localhost:19200/articles", "use es://localhost:19200/articles")]
#[case::elasticsearch_endpoint("https://es.example.com/", "use es+https://es.example.com/<index>")]
#[case::sqlite_file("./notes.db", "use sqlite:./notes.db#<table>")]
fn bad_files(#[case] input: &str, #[case] fragment: &str) {
    let message = input.parse::<File>().unwrap_err().to_string();
    assert!(message.contains(fragment), "got: {message}");
}

#[test]
fn location_display_preserves_identity() {
    let path = std::env::current_dir().unwrap().join("books.csv");
    let path = path.to_str().unwrap();
    for (location, display) in [
        (
            SourceLocation::from(&path.parse::<File>().unwrap()),
            "books.csv",
        ),
        (SourceLocation::Object(path.into()), path),
    ] {
        assert_eq!(location.value(), path);
        assert_eq!(location.to_string(), display);
        let encoded = serde_json::to_string(&location).unwrap();
        assert_eq!(
            serde_json::from_str::<SourceLocation>(&encoded).unwrap(),
            location
        );
    }
}
