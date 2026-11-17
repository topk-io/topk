use crate::import::common::seed::{rustfs, Backend::S3};
use futures::TryStreamExt;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::source::{Checkpoint, Cursor};
use topk::import::Error;
use topk_rs::proto::v1::control::field_type_list::ListValueType;
use topk_rs::proto::v1::control::{field_type_matrix::MatrixValueType, FieldSpec};
use topk_rs::proto::v1::data::Value;

use crate::import::common::*;
use topk::import::plan::Target;

#[rstest_ctx(Scratch)]
#[case::decimal_list(
    "SELECT 1 AS id, [1.50, 2.25, 3.00]::DECIMAL(10,2)[] AS prices",
    vec![("prices", FieldSpec::list(false, ListValueType::Float))], json!({"_id": "1", "prices": [1.5, 2.25, 3.0]})
)]
#[case::bytes_as_text(
    "SELECT 1 AS id, 'hello'::BLOB AS a, '42'::BLOB AS b",
    vec![("a", FieldSpec::text(false)), ("b", FieldSpec::text(false))], json!({"_id": "1", "a": "hello", "b": "42"})
)]
#[case::empty_lists(
    "SELECT 1 AS id, []::VARCHAR[] AS names, []::INTEGER[] AS numbers",
    vec![("names", FieldSpec::list(false, ListValueType::String)), ("numbers", FieldSpec::list(false, ListValueType::Integer))], json!({"_id": "1", "names": [], "numbers": []})
)]
async fn decodes(
    ctx: &Scratch,
    #[case] select: &str,
    #[case] fields: Vec<(&'static str, FieldSpec)>,
    #[case] expected: serde_json::Value,
) {
    let from = ctx.parquet("data.parquet", select);
    assert_eq!(
        json_documents(&source(&from).await.unwrap(), &target(&from, "id", fields))
            .await
            .unwrap(),
        [(None, expected)]
    );
}

#[rstest_ctx(Scratch)]
async fn ragged_csv_keeps_its_header(ctx: &Scratch) {
    let from = ctx.file(
        "ragged.csv",
        "id,name,price\n1,apple,1.20\n2,banana\n3,cherry,2.40,extra",
    );
    let docs = json_documents(
        &source(&from).await.unwrap(),
        &target(&from, "id", [("name", FieldSpec::text(false))]),
    )
    .await
    .unwrap();
    assert_eq!(docs.len(), 3);
    assert_eq!(docs[1].1, json!({"_id": "2", "name": "banana"}));
}

#[rstest_ctx(Scratch)]
#[case::unix_lines("\n")]
#[case::windows_lines("\r\n")]
async fn multiline_ragged_csv(ctx: &Scratch, #[case] newline: &str) {
    let from = ctx.file(
        "multiline.csv",
        &format!("name,id,price{newline}\"a{newline}b\",1,1.20{newline}short,2{newline}long,3,2.40,extra{newline}"),
    );
    let docs = json_documents(
        &source(&from).await.unwrap(),
        &target(&from, "id", [("name", FieldSpec::text(false))]),
    )
    .await
    .unwrap();
    assert_eq!(
        docs,
        [
            (None, json!({"_id": "1", "name": format!("a{newline}b")})),
            (None, json!({"_id": "2", "name": "short"})),
            (None, json!({"_id": "3", "name": "long"})),
        ]
    );
}

#[rstest_ctx(Scratch)]
async fn empty_unsupported_list_is_rejected(ctx: &Scratch) {
    let from = ctx.parquet("data.parquet", "SELECT 1 AS id, []::BOOLEAN[] AS flags");
    let error = json_documents(
        &source(&from).await.unwrap(),
        &target(
            &from,
            "id",
            [("flags", FieldSpec::list(false, ListValueType::Float))],
        ),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("list of Boolean has no equivalent"),
        "{error}"
    );
}

#[rstest_ctx(Scratch)]
async fn bad_filter_names_the_filter(ctx: &Scratch) {
    let from = ctx.seed_parquet("books", books());
    let mut target = target(&from, "_id", [("title", FieldSpec::text(false))]);
    target.filter = Some("bogus(".into());
    let error = json_documents(&source(&from).await.unwrap(), &target)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("bogus("), "{error}");
    assert!(!error.contains("SELECT"), "{error}");
}

#[rstest_ctx(Scratch)]
async fn inferred_types(ctx: &Scratch) {
    let source = ctx.parquet(
        "data.parquet",
        "SELECT 1 AS id, [0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8]::FLOAT[] AS embedding, \
         DATE '2024-01-15' AS d, TIMESTAMP '2024-06-30 12:00:00' AS ts, \
         {'indices': [3,1]::UINTEGER[], 'values': [1.5,0.5]::FLOAT[]} AS sv",
    );

    let connected = crate::import::common::source(&source).await.unwrap();
    let table = connected.discover().await.unwrap();
    let fields: indexmap::IndexMap<_, _> = table
        .columns
        .iter()
        .map(|(name, field)| (name.as_str(), field))
        .collect();

    assert_eq!(
        fields["embedding"],
        &FieldSpec::list(false, ListValueType::Float)
    );
    assert_eq!(fields["d"], &FieldSpec::timestamp(false));
    assert_eq!(fields["ts"], &FieldSpec::timestamp(false));
    assert_eq!(fields["sv"], &FieldSpec::f32_sparse_vector(false));
}

#[rstest_ctx(Scratch)]
async fn ragged_float_lists_stay_lists(ctx: &Scratch) {
    let source = ctx.parquet(
        "data.parquet",
        "SELECT i AS id, CASE WHEN i = 0 THEN [0.1,0.2] ELSE [0.3] END::FLOAT[] AS embedding \
         FROM range(3) t(i)",
    );
    let connected = crate::import::common::source(&source).await.unwrap();
    let table = connected.discover().await.unwrap();
    let field = &table
        .columns
        .iter()
        .find(|(name, _)| name == "embedding")
        .unwrap()
        .1;

    assert_eq!(field, &FieldSpec::list(false, ListValueType::Float));
}

#[rstest_ctx(Scratch)]
async fn read_errors_name_the_source(ctx: &Scratch) {
    let missing = format!("{}/missing-*.parquet", ctx.scratch().display());

    let connected = source(&missing).await.unwrap();
    let message = refused(connected.discover().await);
    assert!(
        message.contains("reading") && message.contains("missing-*.parquet"),
        "got: {message}"
    );
    assert!(message.contains("No files found"), "got: {message}");
}

#[rstest_ctx(Scratch)]
async fn glob_is_one_object(ctx: &Scratch) {
    for i in 0..2 {
        ctx.seed_parquet(&format!("part_{i}"), books());
    }
    let pattern = format!("{}/part_*.parquet", ctx.scratch().display());
    let connected = source(&pattern).await.unwrap();
    let table = connected.discover().await.unwrap();
    assert_eq!(table.from.value(), pattern);
}

#[rstest_ctx(Scratch)]
async fn anonymous_http_reads(ctx: &Scratch) {
    let object = ctx.seed(S3, "books", books()).await;
    // Public-read policy so the same object is a plain anonymous http GET.
    std::process::Command::new("curl")
        .args([
            "-s", "-X", "PUT",
            &format!("{}/topk-it/?policy", rustfs::ENDPOINT),
            "--aws-sigv4", "aws:amz:us-east-1:s3",
            "--user", "rustfsadmin:rustfsadmin",
            "-d",
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:GetObject"],"Resource":["arn:aws:s3:::topk-it/*"]}]}"#,
        ])
        .output()
        .unwrap();
    let from = object.from.replace("s3:/", rustfs::ENDPOINT);
    let source = source(&from).await.unwrap();
    let target = target(&from, "_id", [("title", FieldSpec::text(false))]);
    let docs = json_documents(&source, &target).await.unwrap();
    assert_eq!(docs.len(), 3);
    assert!(docs
        .iter()
        .any(|(_, doc)| doc == &json!({"_id": "mockingbird", "title": "To Kill a Mockingbird"})));
}

#[rstest_ctx(Scratch)]
async fn nan_value_rejected(ctx: &Scratch) {
    let from = ctx.parquet("data.parquet", "SELECT 1 AS id, 'nan'::DOUBLE AS value");
    let source = source(&from).await.unwrap();
    let target = target(&from, "id", [("value", FieldSpec::float(false))]);
    assert!(refused(json_documents(&source, &target).await).contains("non-finite"));
}

#[rstest_ctx(Scratch)]
async fn nested_list_decoding(ctx: &Scratch) {
    let from = ctx.file("matrix.jsonl", r#"{"id":"one","v":[[1,2],[3,4]]}"#);
    let source = source(&from).await.unwrap();
    let target = target(
        &from,
        "id",
        [("v", FieldSpec::matrix(false, 2, MatrixValueType::F32))],
    );
    let docs: Vec<_> = source
        .documents(&target)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    assert_eq!(
        docs,
        [(
            None,
            topk_rs::doc!("_id" => "one", "v" => Value::matrix(2, vec![1_f32, 2., 3., 4.]))
        )]
    );
}

#[rstest_ctx(Scratch)]
async fn blank_csv_id(ctx: &Scratch) {
    let from = ctx.file("blank-id.csv", "id,name\n1,ok\n,blank-id");
    let source = source(&from).await.unwrap();
    let target = target(&from, "id", [("name", FieldSpec::text(false))]);
    assert!(refused(json_documents(&source, &target).await).contains(r#"field "id""#));
}

#[rstest_ctx(Scratch)]
async fn huggingface_image_column(ctx: &Scratch) {
    let from = ctx.parquet(
        "images.parquet",
        "SELECT 1 AS id, {'bytes': from_hex('89504e470d0a1a0a'), 'path': 'page.png'} AS image",
    );
    let connected = source(&from).await.unwrap();
    let target = Target::new(connected.discover().await.unwrap(), Some("id")).unwrap();
    assert_eq!(target.fields["image"].schema, FieldSpec::image(false));
    let docs = connected
        .documents(&target)
        .await
        .unwrap()
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    assert_eq!(
        docs[0].1.fields["image"],
        topk_rs::proto::v1::data::Value::png(b"\x89PNG\r\n\x1A\n".as_slice())
    );
}

#[rstest_ctx(Scratch)]
#[case::whole_file(None, None, 0)]
#[case::limited_rows(Some(25_001), None, 0)]
#[case::filtered_rows(None, Some("a > 25000"), 0)]
#[case::resumed_rows(None, None, 25_000)]
async fn csv_reader_errors_stop_stream(
    ctx: &Scratch,
    #[case] limit: Option<u64>,
    #[case] filter: Option<&str>,
    #[case] offset: u64,
) {
    let mut csv = String::from("id,a,b\n");
    for i in 0..25_000 {
        csv.push_str(&format!("{i},{i},{i}\n"));
    }
    csv.push_str("25000,bad,bad\n25001,25001,25001\n");
    let from = ctx.file("late.csv", &csv);
    let connected = source(&from).await.unwrap();
    let mut target = target(
        &from,
        "id",
        [
            ("a", FieldSpec::integer(false)),
            ("b", FieldSpec::integer(false)),
        ],
    );
    target.limit = limit;
    target.filter = filter.map(str::to_string);
    let checkpoint = Checkpoint {
        cursor: Cursor::Offset {
            part: from,
            rows: offset,
        },
        consumed: offset,
    };
    let mut stream = connected
        .scan(&target, (offset != 0).then_some(&checkpoint))
        .await
        .unwrap();
    loop {
        match stream.try_next().await {
            Err(Error::InvalidArgument(message)) => {
                assert!(message.contains("late.csv"), "{message}");
                break;
            }
            Err(error) => panic!("unexpected error: {error}"),
            Ok(Some(_)) => {}
            Ok(None) => panic!("reader error was swallowed"),
        }
    }
}

#[rstest_ctx(Scratch)]
async fn csv_sniffed_timestamp_filter(ctx: &Scratch) {
    let from = ctx.file(
        "dates.csv",
        "id,date,value\n1,01/02/2024 10:00:00,5\n2,02/02/2024 10:00:00,6\n",
    );
    let connected = source(&from).await.unwrap();
    let mut target = target(
        &from,
        "id",
        [
            ("date", FieldSpec::timestamp(false)),
            ("value", FieldSpec::integer(false)),
        ],
    );
    target.filter = Some("value > 5 AND date > TIMESTAMP '2024-02-01'".into());
    let docs = json_documents(&connected, &target).await.unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].1["_id"], "2");
    assert!(docs[0].1["date"].as_i64().is_some());
}

#[rstest_ctx(Scratch)]
#[case::exact_name("__topk_error")]
#[case::uppercase_name("__TOPK_ERROR")]
async fn csv_excludes_unread_columns(ctx: &Scratch, #[case] name: &str) {
    let mut csv = format!("id,{name},n\n");
    for i in 0..25_000 {
        csv.push_str(&format!("{i},kept,{i}\n"));
    }
    csv.push_str("25000,kept,bad\n");
    let from = ctx.file("excluded.csv", &csv);
    let connected = source(&from).await.unwrap();
    let mut target = Target::new(connected.discover().await.unwrap(), Some("id")).unwrap();
    target
        .select(&connected, &[], &[], &["n".into()], None)
        .await
        .unwrap();
    let docs = json_documents(&connected, &target).await.unwrap();
    assert_eq!(docs.len(), 25_001);
    assert_eq!(
        docs.last().unwrap().1,
        json!({"_id": "25000", (name.trim_start_matches('_')): "kept"})
    );
}
