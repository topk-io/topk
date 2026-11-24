use crate::import::common::seed::{rustfs, Backend::S3};
use futures::TryStreamExt;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::source::{Cursor, ReadEvent, Resume};
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
    let mut target = target(&from, "id", [("name", FieldSpec::text(false))]);
    target.exclude = vec!["price".into(), "column3".into()];
    let docs = json_documents(&source(&from).await.unwrap(), &target)
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
    let mut target = target(&from, "id", [("name", FieldSpec::text(false))]);
    target.exclude = vec!["price".into(), "column3".into()];
    let docs = json_documents(&source(&from).await.unwrap(), &target)
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
#[case::extra_field(&["id,name\n1,a\n2,b,x\n"], "id", &[], &[], Some("column2 has no name"))]
#[case::trailing_comma(&["id,name,\n1,a,\n2,b,\n"], "id", &[], &[], Some("column2 has no name"))]
#[case::padded_names(&["a,b\n1,2,3,4,5,6,7,8,9,10,11,12\n"], "a", &[], &[], Some("column02, column03"))]
#[case::glob_union(&["id,a,b\n1,2,3\n", "id,,c\n4,5,6\n"], "id", &[], &[], Some("column1 has no name"))]
#[case::excluded(&["id,name\n1,a\n2,b,x\n"], "id", &[], &["column2"], None)]
#[case::renamed(&["id,name\n1,a\n2,b,x\n"], "id", &["column2=extra"], &[], None)]
#[case::headerless(&["1,a,1\n2,b,2\n"], "column0", &[], &[], None)]
async fn unnamed_csv_columns(
    ctx: &Scratch,
    #[case] files: &[&str],
    #[case] id: &str,
    #[case] renames: &[&str],
    #[case] exclude: &[&str],
    #[case] error: Option<&str>,
) {
    let paths: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(n, csv)| ctx.file(&format!("csv/{n}.csv"), csv))
        .collect();
    let from = match paths.as_slice() {
        [one] => one.clone(),
        _ => ctx.path("csv/*.csv").to_str().unwrap().to_string(),
    };
    let source = source(&from).await.unwrap();
    let mut target = Target::new(source.discover().await.unwrap(), Some(id)).unwrap();
    let renames: Vec<_> = renames
        .iter()
        .map(|rename| rename.parse().unwrap())
        .collect();
    let exclude: Vec<_> = exclude.iter().map(|name| name.to_string()).collect();
    let result = target.select(&source, &[], &renames, &exclude, None).await;
    match error {
        Some(error) => assert!(refused(result).contains(error)),
        None => result.unwrap(),
    }
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
    assert_eq!(
        message.matches("missing-*.parquet").count(),
        1,
        "got: {message}"
    );
    assert!(
        message.contains("No files found") && !message.contains("LINE "),
        "got: {message}"
    );
}

#[rstest_ctx(Scratch)]
#[case::named_segment("{dir}/{lang}/*.parquet", "{lang}")]
#[case::hive_folder("{dir}/*/*.parquet", "lang={lang}")]
async fn partition_from_path(ctx: &Scratch, #[case] pattern: &str, #[case] folder: &str) {
    for (lang, id) in [("sw", 1), ("yo", 2)] {
        ctx.parquet(
            &format!("{}/part-0.parquet", folder.replace("{lang}", lang)),
            &format!("SELECT '{id}' AS id, 't{id}' AS text"),
        );
    }
    let from = pattern.replace(
        "{dir}",
        &ctx.scratch().display().to_string().replace('\\', "/"),
    );
    let connected = source(&from).await.unwrap();
    let table = connected.discover().await.unwrap();
    assert_eq!(table.from.value(), from);
    assert!(table.columns.iter().any(|(name, _)| name == "lang"));
    let mut target = target(&from, "id", [("text", FieldSpec::text(false))]);
    target.partition = Some("lang".into());
    target
        .select(&connected, &[], &[], &[], None)
        .await
        .unwrap();
    assert_eq!(
        json_documents(&connected, &target).await.unwrap(),
        [
            (Some("sw".into()), json!({"_id": "1", "text": "t1"})),
            (Some("yo".into()), json!({"_id": "2", "text": "t2"})),
        ]
    );
}

#[rstest_ctx(Scratch)]
#[case::matched_folder("{dir}/*/p.parquet", &["1", "2", "6"])]
#[case::matched_file("{dir}/d/*.parquet", &["3", "4"])]
#[case::literal_file("{dir}/d/x{1}.parquet", &["3"])]
#[case::beside_path_column("{dir}/{lang}/x{1}.parquet", &["3", "5"])]
#[case::escaped_folder("{dir}/[{]e}/p.parquet", &["6"])]
async fn braces_are_literal(ctx: &Scratch, #[case] pattern: &str, #[case] ids: &[&str]) {
    for (id, file) in [
        ("1", "a{1}/p.parquet"),
        ("2", "b/p.parquet"),
        ("3", "d/x{1}.parquet"),
        ("4", "d/y.parquet"),
        ("5", "sw/x{1}.parquet"),
        ("6", "{e}/p.parquet"),
    ] {
        ctx.parquet(file, &format!("SELECT '{id}' AS id, 't' AS text"));
    }
    let from = pattern.replace("{dir}", &ctx.scratch().display().to_string());
    let docs = json_documents(
        &source(&from).await.unwrap(),
        &target(&from, "id", [("text", FieldSpec::text(false))]),
    )
    .await
    .unwrap();
    let mut read: Vec<_> = docs
        .iter()
        .map(|(_, doc)| doc["_id"].as_str().unwrap())
        .collect();
    read.sort();
    assert_eq!(read, ids);
}

#[rstest_ctx(Scratch)]
#[case::file_column("sw", ", 'x' AS lang", "{lang}")]
#[case::hive_folder("lang=sw", "", "{lang}")]
#[case::other_case("sw", ", 'x' AS lang", "{Lang}")]
async fn path_column_clash(
    ctx: &Scratch,
    #[case] folder: &str,
    #[case] column: &str,
    #[case] segment: &str,
) {
    ctx.parquet(
        &format!("{folder}/p.parquet"),
        &format!("SELECT '1' AS id{column}"),
    );
    let from = format!("{}/{segment}/*.parquet", ctx.scratch().display());
    let error = refused(source(&from).await.unwrap().discover().await);
    assert!(error.contains("already have a"), "{error}");
}

#[rstest_ctx(Scratch)]
#[case::whole_batch(u64::MAX, false)]
#[case::cut_by_bytes(64 * 1024, true)]
async fn batches_travel_in_chunks(ctx: &Scratch, #[case] chunk_bytes: u64, #[case] cut: bool) {
    let path = ctx.parquet(
        "wide.parquet",
        "SELECT range::VARCHAR AS id, repeat('x', 16384) AS text FROM range(64)",
    );
    let from = format!("file://{path}");
    let target = target(&from, "id", [("text", FieldSpec::text(false))]);
    let mut source = source(&from).await.unwrap();
    source.readers.chunk_bytes = chunk_bytes;
    let events: Vec<_> = source
        .scan(&target, None)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let mut chunks: Vec<_> = events
        .into_iter()
        .map(|event| match event {
            ReadEvent::Chunk(chunk) => (chunk.rows.len() as u64, chunk.cursor, chunk.last),
            ReadEvent::Restarted => panic!("unexpected restart"),
        })
        .collect();
    let end = |rows| {
        Some(Cursor::Offset {
            part: path.clone(),
            rows,
        })
    };
    assert_eq!(chunks.pop(), Some((0, end(64), true)));
    assert_eq!(chunks.remove(0), (0, end(0), false));
    assert_eq!(chunks.len() > 1, cut, "{chunks:?}");
    let mut read = 0;
    for (rows, cursor, last) in chunks {
        read += rows;
        assert!(rows <= if cut { 8 } else { 64 }, "{rows} rows in one chunk");
        assert_eq!((cursor, last), (end(read), false));
    }
    assert_eq!(read, 64);
}

#[rstest_ctx(Scratch)]
#[case::one_reader(1)]
#[case::three_readers(3)]
async fn files_read_side_by_side(ctx: &Scratch, #[case] count: usize) {
    let parts: Vec<String> = (0..3)
        .map(|n| {
            ctx.parquet(
                &format!("many/part-{n}.parquet"),
                &format!("SELECT ({n} * 100 + range)::VARCHAR AS id, 't' AS text FROM range(100)"),
            )
        })
        .collect();
    let from = ctx.path("many/*.parquet").to_str().unwrap().to_string();
    let mut source = source(&from).await.unwrap();
    source.readers.count = count;
    let target = Target::new(source.discover().await.unwrap(), Some("id")).unwrap();
    let events: Vec<_> = source
        .scan(&target, None)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let mut opened = Vec::new();
    let mut read = std::collections::HashMap::new();
    for event in events {
        let chunk = match event {
            ReadEvent::Chunk(chunk) => chunk,
            ReadEvent::Restarted => panic!("unexpected restart"),
        };
        let (part, rows) = match chunk.cursor {
            Some(Cursor::Offset { part, rows }) => (part, rows),
            cursor => panic!("{cursor:?} is not a file offset"),
        };
        let (so_far, last) = read.entry(part.clone()).or_insert_with(|| {
            opened.push(part.clone());
            (0, false)
        });
        assert!(!*last, "{part} read after its last chunk");
        *so_far += chunk.rows.len() as u64;
        assert_eq!(*so_far, rows);
        *last = chunk.last;
    }
    assert_eq!(opened, parts);
    assert!(read.values().all(|&(rows, last)| rows == 100 && last));
}

#[rstest_ctx(Scratch)]
#[case::every_row(None, 6)]
#[case::across_files(Some(3), 3)]
#[case::exact_file(Some(2), 2)]
async fn parallel_reader_limits(ctx: &Scratch, #[case] limit: Option<u64>, #[case] rows: usize) {
    for file in ["a", "b", "c"] {
        ctx.parquet(
            &format!("limited/{file}.parquet"),
            &format!("SELECT '{file}' || range::VARCHAR AS id, 't' AS text FROM range(2)"),
        );
    }
    let from = ctx.path("limited/*.parquet").display().to_string();
    let mut source = source(&from).await.unwrap();
    source.readers.count = 3;
    let mut target = Target::new(source.discover().await.unwrap(), Some("id")).unwrap();
    target.limit = limit;
    let documents = json_documents(&source, &target).await.unwrap();
    assert_eq!(documents.len(), rows);
    if limit.is_some() {
        let ids: Vec<_> = documents
            .iter()
            .map(|(_, doc)| doc["_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["a0", "a1", "b0"][..rows]);
    }
}

#[rstest_ctx(Scratch)]
async fn parallel_reader_errors(ctx: &Scratch) {
    for file in ["a", "c"] {
        ctx.parquet(
            &format!("broken/{file}.parquet"),
            "SELECT range::VARCHAR AS id, 't' AS text FROM range(100)",
        );
    }
    let bad = ctx.file("broken/b.parquet", "invalid parquet");
    let from = ctx.path("broken/*.parquet").display().to_string();
    let mut source = source(&from).await.unwrap();
    source.readers.count = 3;
    source.readers.sample_files = 2;
    let target = Target::new(source.discover().await.unwrap(), Some("id")).unwrap();
    let error = refused(
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            json_documents(&source, &target),
        )
        .await
        .unwrap(),
    );
    assert!(error.contains(&bad), "{error}");
}

#[rstest_ctx(Scratch)]
#[case::every_file(usize::MAX, true)]
#[case::sampled_ends(2, false)]
async fn unseen_columns_pass_through(
    ctx: &Scratch,
    #[case] sample_files: usize,
    #[case] planned: bool,
) {
    for (file, extra) in [("a", ""), ("b", ", 7 AS extra"), ("c", "")] {
        ctx.parquet(
            &format!("mixed/{file}.parquet"),
            &format!("SELECT '{file}' AS id, 't' AS text{extra}"),
        );
    }
    let from = ctx.path("mixed/*.parquet").to_str().unwrap().to_string();
    let mut source = source(&from).await.unwrap();
    source.readers.sample_files = sample_files;
    let table = source.discover().await.unwrap();
    assert_eq!(table.complete, planned);
    let target = Target::new(table, Some("id")).unwrap();
    assert_eq!(target.fields.contains_key("extra"), planned);
    let documents = json_documents(&source, &target).await.unwrap();
    let extras: Vec<_> = documents
        .iter()
        .map(|(_, document)| {
            (
                document["_id"].as_str().unwrap(),
                document.get("extra").cloned(),
            )
        })
        .collect();
    assert_eq!(
        extras,
        [("a", None), ("b", Some(json!(7))), ("c", None)],
        "{documents:?}"
    );
}

#[rstest_ctx(Scratch)]
#[case::explicit_type(&["extra=text"], &[], &[], json!({"_id": "b", "text": "t", "extra": "7"}))]
#[case::renamed_field(&["more=int"], &["extra=more"], &[], json!({"_id": "b", "text": "t", "more": 7}))]
#[case::excluded_field(&[], &[], &["extra"], json!({"_id": "b", "text": "t"}))]
async fn sampled_field_options(
    ctx: &Scratch,
    #[case] fields: &[&str],
    #[case] renames: &[&str],
    #[case] exclude: &[&str],
    #[case] expected: serde_json::Value,
) {
    for (file, extra) in [("a", ""), ("b", ", 7 AS extra"), ("c", "")] {
        ctx.parquet(
            &format!("sampled/{file}.parquet"),
            &format!("SELECT '{file}' AS id, 't' AS text{extra}"),
        );
    }
    let from = ctx.path("sampled/*.parquet").display().to_string();
    let mut source = source(&from).await.unwrap();
    source.readers.sample_files = 2;
    let mut target = Target::new(source.discover().await.unwrap(), Some("id")).unwrap();
    target
        .select(
            &source,
            &fields
                .iter()
                .map(|field| field.parse().unwrap())
                .collect::<Vec<_>>(),
            &renames
                .iter()
                .map(|rename| rename.parse().unwrap())
                .collect::<Vec<_>>(),
            &exclude
                .iter()
                .map(|column| column.to_string())
                .collect::<Vec<_>>(),
            None,
        )
        .await
        .unwrap();
    let documents = json_documents(&source, &target).await.unwrap();
    assert_eq!(
        documents
            .iter()
            .find(|(_, doc)| doc["_id"] == "b")
            .unwrap()
            .1,
        expected
    );
}

#[rstest_ctx(Scratch)]
async fn file_scheme_resumes(ctx: &Scratch) {
    let path = ctx.parquet(
        "rows.parquet",
        "SELECT range::VARCHAR AS id, 't' AS text FROM range(3)",
    );
    let from = format!("file://{path}");
    let target = target(&from, "id", [("text", FieldSpec::text(false))]);
    let checkpoint = Resume {
        open: vec![Cursor::Offset {
            part: path,
            rows: 2,
        }],
        done: Vec::new(),
        consumed: 2,
    };
    let events: Vec<_> = source(&from)
        .await
        .unwrap()
        .scan(&target, Some(&checkpoint))
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let rows: usize = events
        .iter()
        .map(|event| match event {
            ReadEvent::Chunk(chunk) => chunk.rows.len(),
            ReadEvent::Restarted => 0,
        })
        .sum();
    assert_eq!(rows, 1);
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
    let checkpoint = Resume {
        open: vec![Cursor::Offset {
            part: from,
            rows: offset,
        }],
        done: Vec::new(),
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
