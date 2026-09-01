use std::collections::HashMap;

use crate::common::seed::{self, minio, sqlite, Seed};
use crate::common::*;
use indexmap::IndexMap;
use serde_json::json;
use test_context::test_context;
use topk::import::{Field, Spec, Target};
use topk_rs::doc;
use topk_rs::proto::v1::control::field_index::Index as SchemaIndex;
use topk_rs::proto::v1::control::KeywordIndexType;
use topk_rs::proto::v1::control::{FieldIndex, FieldSpec, VectorDistanceMetric};
use topk_rs::proto::v1::data::Value;

fn outcome(stdout: &str, collection: &str) -> serde_json::Value {
    let summary: serde_json::Value = serde_json::from_str(stdout).expect("json summary");
    summary[collection].clone()
}

fn keyword_title() -> IndexMap<String, Field> {
    fields_toml(r#"title = { type = "text", index = "keyword" }"#)
}

#[test_context(Ctx)]
#[tokio::test]
async fn all_types(ctx: &mut Ctx) {
    let path = ctx.sql_parquet(
        "types",
        "SELECT 1 AS id, 42::INTEGER AS n, '\\xDE\\xAD\\xBE\\xEF'::BLOB AS blob, \
         {'a': 1, 'b': 'x'} AS st, [1,2,3]::INTEGER[] AS li, [0.5,1.5]::DOUBLE[] AS lf, \
         ['p','q']::VARCHAR[] AS ls, [0.1,0.2,0.3]::DOUBLE[] AS vf, \
         [0.1,0.2,0.3]::DOUBLE[] AS vf16src, [0.1,0.2,0.3]::DOUBLE[] AS vf8src, \
         [1,2,3]::INTEGER[] AS vint, [1,2,3]::INTEGER[] AS vint2, \
         '{\"1\": 0.5, \"3\": 1.5}' AS sv",
    );
    let collection = ctx.collection("types");
    let toml = format!(
        "[{collection}]
from = {path:?}

id = \"id\"

[{collection}.fields]
n = {{ type = \"int\" }}
blob = {{ type = \"bytes\" }}
st = {{ type = \"struct\" }}
li = {{ type = \"int_list\" }}
lf = {{ type = \"float_list\" }}
ls = {{ type = \"text_list\" }}
vcos = {{ from = \"vf\", type = \"f32_vector\", dim = 3, index = {{ vector = {{ metric = \"cosine\" }} }} }}
vf16 = {{ from = \"vf16src\", type = \"f16_vector\", dim = 3 }}
vf8 = {{ from = \"vf8src\", type = \"f8_vector\", dim = 3 }}
vu8 = {{ from = \"vint\", type = \"u8_vector\", dim = 3 }}
vi8 = {{ from = \"vint2\", type = \"i8_vector\", dim = 3 }}
sv = {{ type = \"f32_sparse_vector\" }}
"
    );
    ok(&["import", "-f", &ctx.spec_file(&toml), "--yes"], &[]);

    let got = ctx.get(&collection, &["1"]).await;
    let d = &got["1"];
    assert_eq!(field(d, "n"), json!(42));
    assert_eq!(field(d, "blob"), json!([222, 173, 190, 239]));
    assert_eq!(field(d, "st"), json!({"a": 1, "b": "x"}));
    assert_eq!(field(d, "li"), json!([1, 2, 3]));
    assert_eq!(field(d, "lf"), json!([0.5, 1.5]));
    assert_eq!(field(d, "ls"), json!(["p", "q"]));
    assert_eq!(field(d, "vu8"), json!([1, 2, 3]));
    assert_eq!(field(d, "vi8"), json!([1, 2, 3]));
    for v in ["vcos", "vf16", "vf8"] {
        assert_eq!(field(d, v).as_array().unwrap().len(), 3, "{v}");
    }
    assert!(d.contains_key("sv"), "sparse vector must be stored");

    let coll = ctx.client().collections().get(&collection).await.unwrap();
    assert!(matches!(
        coll.schema["vcos"]
            .index
            .as_ref()
            .and_then(|i| i.index.as_ref()),
        Some(SchemaIndex::VectorIndex(_))
    ));
}

#[test_context(Ctx)]
#[tokio::test]
async fn matrix_maxsim(ctx: &mut Ctx) {
    let path = ctx.sql_parquet(
        "mat",
        "SELECT 1 AS id, [[0.1,0.2,0.3],[0.4,0.5,0.6]]::DOUBLE[][] AS mat",
    );
    let collection = ctx.collection("matrix");
    let toml = format!(
        "[{collection}]
from = {path:?}

id = \"id\"

[{collection}.fields]
mat = {{ type = \"f32_matrix\", cols = 3, index = {{ multi_vector = {{}} }} }}
"
    );
    ok(&["import", "-f", &ctx.spec_file(&toml), "--yes"], &[]);

    let got = ctx.get(&collection, &["1"]).await;
    assert_eq!(field(&got["1"], "mat").as_array().unwrap().len(), 2);

    let coll = ctx.client().collections().get(&collection).await.unwrap();
    assert!(matches!(
        coll.schema["mat"]
            .index
            .as_ref()
            .and_then(|i| i.index.as_ref()),
        Some(SchemaIndex::MultiVectorIndex(_))
    ));
}

// A binary vector's source is its bytes as a `u8` list, matching the `Vec<u8>`
// the other SDKs take; TopK's `dim` is the byte count.
#[test_context(Ctx)]
#[tokio::test]
async fn binary_vector(ctx: &mut Ctx) {
    let collection = ctx.collection("binary");
    let spec = ctx.target_spec(
        &collection,
        target(
            &ctx.sql_parquet("bin", "SELECT 1 AS id, [1,2,3,4]::INTEGER[] AS bits"),
            "id",
            r#"bits = { type = "binary_vector", dim = 4 }"#,
        ),
    );
    ok(&["import", "-f", &spec, "--yes"], &[]);
    let got = ctx.get(&collection, &["1"]).await;
    assert_eq!(field(&got["1"], "bits"), json!([1, 2, 3, 4]));
}

#[test_context(Ctx)]
#[tokio::test]
async fn exact_index(ctx: &mut Ctx) {
    let object = ctx.seed_parquet("books", books()).await;
    let collection = ctx.collection("exact");
    let spec = ctx.target_spec(
        &collection,
        Target {
            fields: fields_toml(r#"title = { type = "text", required = true, index = "exact" }"#),
            ..object
        },
    );
    ok(&["import", "-f", &spec, "--yes"], &[]);

    let coll = ctx.client().collections().get(&collection).await.unwrap();
    assert!(coll.schema["title"].required);
    assert!(matches!(
        coll.schema["title"].index.as_ref().and_then(|i| i.index.as_ref()),
        Some(SchemaIndex::KeywordIndex(k)) if k.index_type == KeywordIndexType::Exact as i32
    ));
}

#[test_context(Ctx)]
#[tokio::test]
async fn batching(ctx: &mut Ctx) {
    let collection = ctx.collection("batch");
    // 2500 rows x ~4KB > BATCH_BYTES (8MB) → at least 2 upserts
    let docs = (1..=2500_u64)
        .map(|i| doc!("_id" => i.to_string(), "pad" => "x".repeat(4096)))
        .collect();
    let object = ctx.seed_parquet("big", docs).await;
    let spec = ctx.target_spec(&collection, object);
    ok(&["import", "-f", &spec, "--yes"], &[]);

    let got = ctx.get(&collection, &["1", "2000", "2500"]).await;
    assert_eq!(got.len(), 3, "rows from both batches must be present");
}

#[test_context(Ctx)]
#[tokio::test]
async fn duplicate_ids(ctx: &mut Ctx) {
    let collection = ctx.collection("dups");
    let docs = vec![
        doc!("_id" => "a", "title" => "one"),
        doc!("_id" => "b", "title" => "two"),
        doc!("_id" => "a", "title" => "three"),
    ];
    let object = ctx.seed_parquet("dups", docs).await;
    let spec = ctx.target_spec(&collection, object);

    let run = outcome(
        &ok(&["import", "-f", &spec, "--yes", "-o", "json"], &[]),
        &collection,
    );
    assert_eq!(run["rows"], json!(3));
    assert_eq!(
        ctx.get(&collection, &["a", "b"]).await.len(),
        2,
        "duplicate ids collapse into one doc"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn multi_collection(ctx: &mut Ctx) {
    let first = ctx.seed_parquet("first", books()).await;
    let second = ctx.seed_parquet("second", rows(1, 3)).await;
    let a = ctx.collection("multi-a");
    let b = ctx.collection("multi-b");

    let spec = ctx.multi_spec([(a.clone(), first), (b.clone(), second)]);
    ok(&["import", "-f", &spec, "--yes"], &[]);

    assert_eq!(ctx.get(&a, &["mockingbird"]).await.len(), 1);
    assert_eq!(ctx.get(&b, &["2"]).await.len(), 1);
}

#[test_context(Ctx)]
#[tokio::test]
async fn partition(ctx: &mut Ctx) {
    let collection = ctx.collection("partition");
    let object = ctx.seed_parquet("partition", books()).await;
    let spec = ctx.target_spec(&collection, object);
    ok(
        &["import", "-f", &spec, "--partition", "acme", "--yes"],
        &[],
    );

    let partitioned = ctx
        .client()
        .collection(&collection)
        .partition("acme")
        .get(
            ["mockingbird"],
            None,
            None,
            Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong),
        )
        .await
        .expect("get from partition");
    assert_eq!(partitioned.len(), 1);
    assert_eq!(
        ctx.get(&collection, &["mockingbird"]).await.len(),
        0,
        "the default partition stays empty"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn empty_region_reads_as_missing(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let spec = ctx.target_spec("empty-region", object);
    let err = fails(&["import", "-f", &spec, "--yes"], &[("TOPK_REGION", "")]);
    assert!(err.contains("--region is required"), "got:\n{err}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn yes_required_without_tty(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let spec = ctx.target_spec("confirm", object);
    let err = fails(&["import", "-f", &spec], &[]);
    assert!(err.contains("pass --yes"), "got:\n{err}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn spec_only_roundtrip(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let printed = discover(&object.from, None);
    let spec: Spec = toml::from_str(&printed).expect("printed spec reads back");
    assert_eq!(spec.collections.len(), 1);
}

// A committed fixture: duckdb's avro extension reads but cannot write.
#[test_context(Ctx)]
#[tokio::test]
async fn avro_roundtrip(ctx: &mut Ctx) {
    let collection = ctx.collection("avro");
    let object = seed::discovered(
        Target {
            from: concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/books.avro").to_string(),
            ..Default::default()
        },
        None,
    )
    .await
    .unwrap();
    let spec = ctx.target_spec(&collection, object);
    ok(&["import", "-f", &spec, "--yes"], &[]);

    let got = ctx.get(&collection, &["mockingbird", "pride"]).await;
    assert_eq!(
        doc_json(&got["mockingbird"]),
        json!({
            "_id": "mockingbird", "title": "To Kill a Mockingbird", "author": "Lee",
            "published_year": 1960, "rating": 4.3, "in_print": true
        })
    );
    assert_eq!(
        doc_json(&got["pride"]),
        json!({
            "_id": "pride", "title": "Pride and Prejudice", "author": "Austen",
            "published_year": 1813, "rating": 4.3, "in_print": false
        })
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn s3_roundtrip(ctx: &mut Ctx) {
    let s3 = minio::S3::new().unwrap();
    let object = s3.seed(&unique_name("s3"), books()).await.unwrap();
    let collection = ctx.collection("s3");
    let spec = ctx.target_spec(&collection, object);
    ok(&["import", "-f", &spec, "--yes"], minio::S3::ENV);
    let got = ctx.get(&collection, &["mockingbird"]).await;
    assert_eq!(
        doc_json(&got["mockingbird"]),
        json!({
            "_id": "mockingbird", "title": "To Kill a Mockingbird", "author": "Lee",
            "published_year": 1960, "rating": 4.3, "in_print": true
        })
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn http_roundtrip(ctx: &mut Ctx) {
    let s3 = minio::S3::new().unwrap();
    let name = unique_name("http");
    let object = s3.seed(&name, books()).await.unwrap();
    // Public-read policy so the same object is a plain anonymous http GET.
    std::process::Command::new("curl")
        .args([
            "-s", "-X", "PUT",
            &format!("{}/topk-it/?policy", minio::S3::ENDPOINT),
            "--aws-sigv4", "aws:amz:us-east-1:s3",
            "--user", "minioadmin:minioadmin",
            "-d",
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:GetObject"],"Resource":["arn:aws:s3:::topk-it/*"]}]}"#,
        ])
        .output()
        .unwrap();

    let object = Target {
        from: format!("{}/topk-it/{name}.parquet", minio::S3::ENDPOINT),
        ..object
    };
    let collection = ctx.collection("http");
    let spec = ctx.target_spec(&collection, object);
    ok(&["import", "-f", &spec, "--yes"], &[]);
    let got = ctx.get(&collection, &["mockingbird"]).await;
    assert_eq!(
        got["mockingbird"]["title"],
        topk_rs::proto::v1::data::Value::string("To Kill a Mockingbird")
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn nan_rejected(ctx: &mut Ctx) {
    let collection = ctx.collection("nan");
    let spec = ctx.target_spec(
        &collection,
        target(
            &ctx.sql_parquet("nan", "SELECT 1 AS id, 'nan'::DOUBLE AS x"),
            "id",
            r#"x = { type = "float" }"#,
        ),
    );
    assert!(fails(&["import", "-f", &spec, "--dry-run"], &[]).contains("non-finite"));
    assert!(fails(&["import", "-f", &spec, "--yes"], &[]).contains("non-finite"));
}

#[test_context(Ctx)]
#[tokio::test]
async fn semantic_index(ctx: &mut Ctx) {
    let db = sqlite::Db::new().unwrap();
    let object = db.seed(&unique_name("books"), books()).await.unwrap();
    let collection = ctx.collection("semantic");

    let spec = ctx.target_spec(
        &collection,
        Target {
            fields: fields_toml(r#"title = { type = "text", index = "semantic" }"#),
            ..object
        },
    );
    ok(&import_args(db.url().as_deref(), &spec, &["--yes"]), &[]);

    let docs = ctx.get(&collection, &["mockingbird"]).await;
    let keys: Vec<&String> = docs["mockingbird"].keys().collect();
    assert!(
        keys.iter().any(|k| k.contains("embedding")),
        "expected a generated embedding field, got {keys:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn schema_drift(ctx: &mut Ctx) {
    let object = ctx.seed_parquet("books", books()).await;
    let collection = ctx.collection("drift");

    let plain = ctx.target_spec(&collection, object.clone());
    ok(&["import", "-f", &plain, "--yes"], &[]);

    let indexed = ctx.target_spec(
        &collection,
        Target {
            fields: keyword_title(),
            ..object
        },
    );
    let err = fails(&["import", "-f", &indexed, "--yes"], &[]);
    assert!(
        err.contains("schema mismatch") && err.contains("title"),
        "expected schema drift error, got:\n{err}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn continue_on_error(ctx: &mut Ctx) {
    let file = ctx.scratch().join("mixed.csv");
    std::fs::write(&file, "id,n\n1,5\n2,not-a-number\n3,7\n").unwrap();
    let collection = ctx.collection("skips");
    let spec = ctx.target_spec(
        &collection,
        target(&file.display().to_string(), "id", r#"n = { type = "int" }"#),
    );

    let err = fails(&["import", "-f", &spec, "--yes"], &[]);
    assert!(err.contains(r#"doc "2" field "n""#), "got:\n{err}");

    // Skipping is opt-in and still exits non-zero, but the good rows land.
    let err = fails(
        &["import", "-f", &spec, "--yes", "--continue-on-error"],
        &[],
    );
    assert!(err.contains("2 rows written, 1 failed"), "got:\n{err}");
    let got = ctx.get(&collection, &["1", "2", "3"]).await;
    assert_eq!(got.len(), 2, "only the bad row is missing");
}

#[test_context(Ctx)]
#[tokio::test]
async fn reimport_is_idempotent(ctx: &mut Ctx) {
    let object = ctx.seed_parquet("books", books()).await;
    let collection = ctx.collection("idempotent");
    let spec = ctx.target_spec(&collection, object);

    for _ in 0..2 {
        let run = outcome(
            &ok(&["import", "-f", &spec, "--yes", "-o", "json"], &[]),
            &collection,
        );
        assert_eq!(run["rows"], json!(books().len()));
    }
    let got = ctx.get(&collection, &["mockingbird", "pride"]).await;
    assert_eq!(got.len(), 2, "a re-run replaces rather than duplicates");
}

#[test_context(Ctx)]
#[tokio::test]
async fn unknown_field_column_fails_before_creating(ctx: &mut Ctx) {
    let file = ctx.scratch().join("books.csv");
    std::fs::write(&file, "id,title\n1,dune\n").unwrap();
    let collection = ctx.collection("ghost-field");
    let spec = ctx.target_spec(
        &collection,
        target(
            &file.display().to_string(),
            "id",
            "title = { type = \"text\" }\n\
             ghost = { type = \"text\", from = \"nonexistent_col\" }",
        ),
    );

    // Without validation this "succeeds", writing a silent null for `ghost`.
    let err = fails(&["import", "-f", &spec, "--yes"], &[]);
    assert!(
        err.contains(r#"reads column "nonexistent_col""#) && err.contains("available: id, title"),
        "got:\n{err}"
    );
    assert!(
        ctx.client().collections().get(&collection).await.is_err(),
        "a bad field column must fail before creating {collection:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn missing_required_field_fails_before_creating(ctx: &mut Ctx) {
    let file = ctx.scratch().join("books.csv");
    std::fs::write(&file, "id,title\n1,dune\n").unwrap();
    let collection = ctx.collection("missing-required");
    let spec = ctx.target_spec(
        &collection,
        target(
            &file.display().to_string(),
            "id",
            "absent = { type = \"text\", required = true }",
        ),
    );

    let err = fails(&["import", "-f", &spec, "--yes"], &[]);
    assert!(
        err.contains(r#"field "absent""#) && err.contains("available: id, title"),
        "got:\n{err}"
    );
    assert!(
        ctx.client().collections().get(&collection).await.is_err(),
        "a missing required field must fail before creating {collection:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn preflight(ctx: &mut Ctx) {
    let first = ctx.seed_parquet("first", books()).await;
    let second = ctx.seed_parquet("second", books()).await;
    let good = ctx.collection("preflight-good");
    let drifted = ctx.collection("preflight-drift");

    let seed = ctx.target_spec(&drifted, second.clone());
    ok(&["import", "-f", &seed, "--yes"], &[]);

    let both = ctx.multi_spec([
        (good.clone(), first),
        (
            drifted.clone(),
            Target {
                fields: keyword_title(),
                ..second
            },
        ),
    ]);
    let err = fails(&["import", "-f", &both, "--yes"], &[]);
    assert!(err.contains("schema mismatch"), "got:\n{err}");

    assert!(
        ctx.client().collections().get(&good).await.is_err(),
        "preflight must fail before creating {good:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn unknown_id_column_fails_before_creating(ctx: &mut Ctx) {
    let file = ctx.scratch().join("books.csv");
    std::fs::write(&file, "id,title\n1,dune\n").unwrap();
    let collection = ctx.collection("bad-id");

    let err = fails(
        &[
            "import",
            &file.display().to_string(),
            "--to",
            &collection,
            "--id",
            "nope",
            "--yes",
        ],
        &[],
    );
    assert!(
        err.contains(r#"id column "nope""#) && err.contains("available: id, title"),
        "got:\n{err}"
    );
    assert!(
        ctx.client().collections().get(&collection).await.is_err(),
        "a bad --id must fail before creating {collection:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn blank_id(ctx: &mut Ctx) {
    let file = ctx.scratch().join("mixed.csv");
    std::fs::write(&file, "id,name\n1,ok\n,blank-id\n").unwrap();
    let collection = ctx.collection("blankid");
    let spec = ctx.target_spec(
        &collection,
        target(
            &file.display().to_string(),
            "id",
            r#"name = { type = "text" }"#,
        ),
    );
    let err = fails(&["import", "-f", &spec, "--yes"], &[]);
    assert!(err.contains(r#"field "id""#), "got:\n{err}");
}

#[test_context(Ctx)]
#[tokio::test]
async fn json_as_struct(ctx: &mut Ctx) {
    let collection = ctx.collection("json");
    let spec = ctx.target_spec(
        &collection,
        target(
            &ctx.sql_parquet(
                "payloads",
                r#"SELECT 1 AS id, '{"k":{"deep":1}}' AS meta, '["a","b"]' AS tags"#,
            ),
            "id",
            r#"meta = { type = "struct" }
               tags = { type = "text_list" }"#,
        ),
    );
    ok(&["import", "-f", &spec, "--yes"], &[]);

    let got = ctx.get(&collection, &["1"]).await;
    assert_eq!(field(&got["1"], "meta"), json!({"k": {"deep": 1}}));
    assert_eq!(field(&got["1"], "tags"), json!(["a", "b"]));
}

/// A run that stops leaves its state behind; `--resume` finishes the job
/// without re-reading what already landed.
#[test_context(Ctx)]
#[tokio::test]
async fn resume_continues_where_upserts_landed(ctx: &mut Ctx) {
    let dir = ctx.scratch().join("parts");
    std::fs::create_dir(&dir).unwrap();
    let conn = duckdb::Connection::open_in_memory().unwrap();
    for (file, lo, hi) in [("a", 0, 300), ("b", 300, 600), ("c", 600, 900)] {
        // One poisoned id in the last file, past its first rows.
        conn.execute_batch(&format!(
            "COPY (SELECT CASE WHEN i = 750 THEN NULL ELSE i END AS id, 'row ' || i AS name \
             FROM range({lo}, {hi}) t(i)) TO '{}' (FORMAT parquet);",
            dir.join(format!("{file}.parquet")).display()
        ))
        .unwrap();
    }
    let glob = format!("{}/*.parquet", dir.display());
    let collection = ctx.collection("parts");

    // Every row its own upsert, so checkpoints land before the failure.
    let stderr = fails(
        &[
            "import",
            &glob,
            "--to",
            &collection,
            "--yes",
            "--batch-bytes",
            "1",
        ],
        &[],
    );
    let run = stderr
        .lines()
        .find_map(|l| l.strip_prefix("# run "))
        .expect("run id in header")
        .split(',')
        .next()
        .unwrap()
        .to_string();
    assert!(stderr.contains(&format!("--resume {run}")), "{stderr}");
    let state = std::fs::read_to_string(state_dir().join(format!("{run}.toml"))).unwrap();
    // A file this small is one arrow chunk, and a chunk's mark comes after its
    // rows — so the last landed mark is the end of b; c is re-read whole.
    assert!(state.contains("b.parquet:300\""), "{state}");

    // Exits non-zero for the skipped row; the summary is still on stdout.
    let out = crate::common::run(
        &[
            "import",
            &glob,
            "--resume",
            &run,
            "--yes",
            "--batch-bytes",
            "1",
            "--continue-on-error",
            "-o",
            "json",
        ],
        &[],
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let summary = stdout.lines().next().expect("summary line");
    let rows = outcome(summary, &collection)["rows"].as_u64().unwrap();
    assert_eq!(outcome(summary, &collection)["failed"], 1);
    assert_eq!(rows, 299, "resume re-read c.parquet alone");
    assert!(
        !state_dir().join(format!("{run}.toml")).exists(),
        "state is deleted on success"
    );

    let got = ctx
        .get(&collection, &["0", "299", "300", "749", "751", "899"])
        .await;
    assert_eq!(got.len(), 6, "every row but the poisoned one is there");
    assert!(ctx.get(&collection, &["750"]).await.is_empty());
}

/// An upsert replaces the document, so a spec that drops a field clears it on
/// every row it rewrites — and the drift refusal advises dropping fields. Say so
/// before the prompt.
#[test_context(Ctx)]
#[tokio::test]
async fn narrowing_a_spec_warns_that_rows_lose_fields(ctx: &mut Ctx) {
    let object = ctx.seed_parquet("narrow", books()).await;
    let collection = ctx.collection("narrow");
    ok(
        &[
            "import",
            "-f",
            &ctx.target_spec(&collection, object.clone()),
            "--yes",
        ],
        &[],
    );

    let narrowed = ctx.target_spec(
        &collection,
        Target {
            fields: fields([("title", Field::default())]),
            ..object.clone()
        },
    );
    let out = crate::common::run(&["import", "-f", &narrowed, "--yes"], &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("in the collection but not in this spec"),
        "expected a warning naming the dropped fields, got: {stderr}"
    );

    // The fields stay in the collection's schema, so a spec that declares them
    // all has nothing to clear and says nothing. (`target_spec` rewrites one
    // file, so the full spec has to be written again.)
    let full = ctx.target_spec(&collection, object);
    let out = crate::common::run(&["import", "-f", &full, "--yes"], &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("not in this spec"),
        "a spec matching the collection must stay quiet, got: {stderr}"
    );
}

/// `--limit 0` reads nothing, so it leaves nothing behind — an empty collection
/// would collide with the next run's schema.
#[test_context(Ctx)]
#[tokio::test]
async fn limit_zero_creates_nothing(ctx: &mut Ctx) {
    let object = ctx.seed_parquet("zero", books()).await;
    let collection = ctx.collection("zero");
    let spec = ctx.target_spec(&collection, object);
    let stdout = ok(
        &["import", "-f", &spec, "--yes", "--limit", "0", "-o", "json"],
        &[],
    );
    assert_eq!(
        outcome(stdout.lines().next().unwrap(), &collection)["rows"],
        0
    );
    assert!(
        ctx.client()
            .collections()
            .get(&collection)
            .await
            .is_err_and(|e| matches!(e, topk_rs::Error::CollectionNotFound)),
        "no collection is created"
    );
}

/// A limit cannot be resumed from a cursor — the source would apply it again
/// past the mark and write more rows than were asked for. A limited collection
/// restarts instead.
#[test_context(Ctx)]
#[tokio::test]
async fn limited_run_restarts_rather_than_over_reading(ctx: &mut Ctx) {
    let dir = ctx.scratch().join("limited");
    std::fs::create_dir(&dir).unwrap();
    let conn = duckdb::Connection::open_in_memory().unwrap();
    for (file, lo, hi) in [("a", 0, 300), ("b", 300, 600), ("c", 600, 900)] {
        // Poisoned inside the limit, so the first run stops after a checkpoint.
        conn.execute_batch(&format!(
            "COPY (SELECT CASE WHEN i = 450 THEN NULL ELSE i END AS id, 'row ' || i AS name \
             FROM range({lo}, {hi}) t(i)) TO '{}' (FORMAT parquet);",
            dir.join(format!("{file}.parquet")).display()
        ))
        .unwrap();
    }
    let glob = format!("{}/*.parquet", dir.display());
    let collection = ctx.collection("limited");

    let stderr = fails(
        &[
            "import",
            &glob,
            "--to",
            &collection,
            "--yes",
            "--limit",
            "500",
            "--batch-bytes",
            "1",
        ],
        &[],
    );
    let run = stderr
        .lines()
        .find_map(|l| l.strip_prefix("# run "))
        .expect("run id in header")
        .split(',')
        .next()
        .unwrap()
        .to_string();
    let state = std::fs::read_to_string(state_dir().join(format!("{run}.toml"))).unwrap();
    assert!(!state.contains("after"), "no mark is kept: {state}");

    let out = crate::common::run(
        &[
            "import",
            &glob,
            "--resume",
            &run,
            "--yes",
            "--batch-bytes",
            "1",
            "--continue-on-error",
            "-o",
            "json",
        ],
        &[],
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let summary = stdout.lines().next().expect("summary line");
    assert_eq!(outcome(summary, &collection)["rows"], 499);
    assert_eq!(outcome(summary, &collection)["failed"], 1);

    assert_eq!(ctx.get(&collection, &["0", "499"]).await.len(), 2);
    assert!(
        ctx.get(&collection, &["500", "600"]).await.is_empty(),
        "nothing past the limit"
    );
}

/// A `topk://` copy is lossless where a query is not: an indexed vector reaches
/// the destination bit for bit, and the schema arrives with its indexes.
#[test_context(Ctx)]
#[tokio::test]
async fn topk_source_copies_schema_and_indexed_vectors(ctx: &mut Ctx) {
    let source = ctx.collection("copy-src");
    let target = ctx.collection("copy-dst");
    let schema = HashMap::from([
        (
            "emb".to_string(),
            FieldSpec::f32_vector(8, false)
                .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean)),
        ),
        (
            "title".to_string(),
            FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Text)),
        ),
        ("created".to_string(), FieldSpec::timestamp(false)),
    ]);
    ctx.client()
        .collections()
        .create(&source, schema.clone(), None)
        .await
        .expect("create source");
    let vectors: Vec<Vec<f32>> = (0..50)
        .map(|i| {
            (0..8)
                .map(|j| (i as f32 * 0.137 + j as f32).sin())
                .collect()
        })
        .collect();
    ctx.client()
        .collection(&source)
        .upsert(
            vectors
                .iter()
                .enumerate()
                .map(|(i, vector)| {
                    doc!(
                        "_id" => format!("d{i}"),
                        "title" => format!("t{i}"),
                        "created" => 1_704_164_645_000i64 + i as i64,
                        "emb" => vector.clone()
                    )
                })
                .collect(),
        )
        .await
        .expect("seed source");

    let region = std::env::var("TOPK_REGION").expect("TOPK_REGION not set");
    ok(
        &[
            "import",
            &format!("topk://{region}/{source}"),
            "--to",
            &target,
            "--yes",
        ],
        &[],
    );

    let stored = ctx
        .client()
        .collections()
        .get(&source)
        .await
        .expect("source collection");
    let copied = ctx
        .client()
        .collections()
        .get(&target)
        .await
        .expect("copied collection");
    assert_eq!(
        copied.schema, stored.schema,
        "the schema copies with its indexes"
    );

    let ids: Vec<String> = (0..50).map(|i| format!("d{i}")).collect();
    let got = ctx
        .get(&target, &ids.iter().map(String::as_str).collect::<Vec<_>>())
        .await;
    assert_eq!(got.len(), 50);
    for (i, vector) in vectors.iter().enumerate() {
        let doc = &got[&format!("d{i}")];
        assert_eq!(
            doc.get("emb").and_then(Value::as_f32_list),
            Some(vector.as_slice()),
            "an indexed vector must not come back quantized"
        );
        assert_eq!(
            doc.get("created").and_then(Value::as_i64),
            Some(1_704_164_645_000i64 + i as i64)
        );
    }
}

/// Pages ascend by `_id`, and a stored cursor picks up after it — the two
/// properties `--resume` rests on for this source.
#[test_context(Ctx)]
#[tokio::test]
async fn topk_source_pages_by_id_and_resumes_from_a_cursor(ctx: &mut Ctx) {
    let source = ctx.collection("resume-src");
    let target = ctx.collection("resume-dst");
    ctx.client()
        .collections()
        .create(
            &source,
            HashMap::from([("name".to_string(), FieldSpec::text(false))]),
            None,
        )
        .await
        .expect("create source");
    // Padded: `_id` orders lexicographically, so "d9" would sort after "d10".
    ctx.client()
        .collection(&source)
        .upsert(
            (0..2000)
                .map(|i| doc!("_id" => format!("d{i:04}"), "name" => format!("n{i}")))
                .collect(),
        )
        .await
        .expect("seed source");

    let region = std::env::var("TOPK_REGION").expect("TOPK_REGION not set");
    let uri = format!("topk://{region}/{source}");

    // A limit takes the first ids in order, not an arbitrary thousand.
    ok(
        &["import", &uri, "--to", &target, "--yes", "--limit", "1000"],
        &[],
    );
    let head = ctx.get(&target, &["d0000", "d0999"]).await;
    assert_eq!(head.len(), 2, "the first thousand ids are the lowest ones");
    assert!(ctx.get(&target, &["d1000"]).await.is_empty());

    // Resuming from that boundary writes the tail and nothing before it.
    let run = "aaaa0001";
    let spec = ok(&["import", &uri, "--to", &target, "--dry-run"], &[]);
    std::fs::write(
        state_dir().join(format!("{run}.toml")),
        format!(
            "source = \"topk://{region}/{source}\"\n\
             started = \"2026-01-01T00:00:00Z\"\n\
             spec = \"\"\"\n{spec}\"\"\"\n\n\
             [cursors.\"{target}\"]\n\
             after = \"d0999\"\n"
        ),
    )
    .expect("write resume state");
    let summary = ok(
        &["import", &uri, "--resume", run, "--yes", "-o", "json"],
        &[],
    );
    assert_eq!(outcome(&summary, &target)["rows"], 1000, "only the tail");
    assert_eq!(ctx.get(&target, &["d1999"]).await.len(), 1);
}
