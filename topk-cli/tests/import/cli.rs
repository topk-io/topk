use crate::import::common::seed::mongo;
use crate::import::common::seed::Backend;
use crate::import::common::seed::Backend::{Mongo, Postgres};
use crate::import::common::*;
use base64::Engine;
use clap::{Args, Command};
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;
use test_macros::rstest_ctx;
use topk::commands::import::ImportArgs;
use topk_rs::doc;
use topk_rs::proto::v1::control::{
    field_type_list::ListValueType, field_type_matrix::MatrixValueType, FieldIndex, FieldSpec,
    KeywordIndexType, MultiVectorDistanceMetric, VectorDistanceMetric,
};

const BOOK_IDS: [&str; 3] = ["mockingbird", "nineteen_eighty_four", "pride"];

#[rstest_ctx(Ctx)]
async fn skips_oversized_document(ctx: &Ctx) {
    let collection = ctx.collection("oversized");
    let from = ctx.cli.file(
        "rows.jsonl",
        &[
            json!({"id": "good1", "body": "ok"}).to_string(),
            json!({"id": "bad", "body": "x".repeat(2_000_000)}).to_string(),
            json!({"id": "good2", "body": "ok"}).to_string(),
        ]
        .join("\n"),
    );
    let done = ctx
        .cli
        .command()
        .args([
            "import",
            &from,
            "--to",
            &collection,
            "--id",
            "id",
            "--yes",
            "--continue-on-error",
            "-o",
            "json",
        ])
        .success();
    let outcome = done.summary();
    assert_eq!((outcome.rows, outcome.failed), (2, 1));
    assert_eq!(
        ctx.docs(&collection, ["good1", "bad", "good2"]).await,
        [
            json!({"_id": "good1", "body": "ok"}),
            json!({"_id": "good2", "body": "ok"}),
        ]
    );
}

/// A bare scheme takes its connection from the source's usual environment.
#[rstest_ctx(Ctx)]
#[case::postgres_env(Postgres, "postgres://", &[
    ("PGHOST", "localhost"), ("PGPORT", "5433"), ("PGUSER", "postgres"),
    ("PGPASSWORD", "postgres"), ("PGDATABASE", "demo"),
])]
#[case::mongo_env(Mongo, "mongodb://", &[("MONGODB_URI", mongo::URL)])]
async fn bare_scheme_connects_from_environment(
    ctx: &Ctx,
    #[case] backend: Backend,
    #[case] scheme: &str,
    #[case] env: &[(&str, &str)],
) {
    let ctx = &ctx.cli;
    let source = ctx.seed(backend, "books", books()).await;
    let run = ctx
        .command()
        .envs(env)
        .args([
            "-o",
            "json",
            "import",
            &format!("{scheme}#{}", source.from),
            "--to",
            "c",
            "--dry-run",
        ])
        .success();
    assert_eq!(run.plan().target.from.value(), source.from);
}

#[rstest_ctx(Ctx)]
async fn previews_carry_no_credentials(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let source = ctx.seed(Postgres, "books", books()).await;
    let printed = ctx
        .command()
        .args(["import", &source.url, "--to", "c", "--dry-run"])
        .success()
        .stdout;
    assert!(
        !printed.contains("postgres"),
        "a preview names what to import, never where to connect:\n{printed}"
    );
}

#[rstest_ctx(Ctx)]
async fn deterministic_output(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let first = ctx
        .command()
        .args(["import", &from, "--to", "c", "--dry-run"])
        .success();
    let second = ctx
        .command()
        .args(["import", &from, "--to", "c", "--dry-run"])
        .success();
    assert_eq!(first.stdout, second.stdout);
    assert!(!first.preview().is_empty(), "documents preview on stderr");
    assert_eq!(first.preview(), second.preview());
}

#[rstest_ctx(Ctx)]
#[case::float_list("FLOAT")]
#[case::integer_list("BIGINT")]
#[case::unsigned_list("UBIGINT")]
async fn fields_rename_and_inferred_vector(ctx: &Ctx, #[case] ty: &str) {
    let from = ctx.cli.parquet(
        "fields.parquet",
        &format!("SELECT 'two' AS id, 'Other' AS raw_title, [1::{ty}, 2::{ty}, 3::{ty}] AS embedding, 'unused' AS extra UNION ALL SELECT 'one', 'Dune', [1::{ty}, 2::{ty}], 'unused'"),
    );
    let collection = ctx.collection("fields");
    let args = [
        "import",
        &from,
        "--to",
        &collection,
        "--rename",
        "raw_title=title",
        "--filter",
        "id = 'one'",
        "--exclude",
        "extra",
        "-f",
        "title=keyword,embedding=vector:cosine",
    ];
    let preview = ctx
        .cli
        .command()
        .args(["-o", "json"])
        .args(args)
        .args(["--dry-run"])
        .success();
    let target = &preview.plan().target;
    assert_eq!(
        target.fields.keys().map(String::as_str).collect::<Vec<_>>(),
        ["embedding", "title"]
    );
    assert_eq!(target.fields["title"].from.as_deref(), Some("raw_title"));
    let mut vector = FieldSpec::f32_vector(2, false);
    vector.index = Some(FieldIndex::vector(VectorDistanceMetric::Cosine));
    assert_eq!(target.fields["embedding"].schema, vector);
    let done = ctx
        .cli
        .command()
        .args(args)
        .args(["--yes", "-o", "json"])
        .success();
    assert_eq!(done.summary().rows, 1);
    let schema = ctx
        .client
        .collections()
        .get(&collection)
        .await
        .unwrap()
        .schema;
    assert!(topk::import::plan::Field::new(
        FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Text))
    )
    .matches(&schema["title"]));
    assert_eq!(
        ctx.docs(&collection, ["one"]).await,
        [json!({"_id":"one","title":"Dune","embedding":[1.0,2.0]})]
    );
    ctx.cli.command().args(args).args(["--yes"]).success();
    let mismatch = ctx
        .cli
        .command()
        .args([
            "import",
            &from,
            "--to",
            &collection,
            "-f",
            "embedding=vector:2:euclidean",
            "--yes",
        ])
        .failure(1);
    assert!(mismatch.stderr.contains("want"), "{mismatch:?}");
}

#[rstest_ctx(Ctx)]
#[case::text("'hello'", "value=text", FieldSpec::text(false), json!("hello"))]
#[case::int("'42'", "value=int", FieldSpec::integer(false), json!(42))]
#[case::float("'1.5'", "value=float", FieldSpec::float(false), json!(1.5))]
#[case::bool("'true'", "value=bool", FieldSpec::boolean(false), json!(true))]
#[case::bytes("CAST('abc' AS BLOB)", "value=bytes", FieldSpec::bytes(false), json!([97, 98, 99]))]
#[case::image("'data:image/png;base64,iVBORw0KGgo='", "value=image", FieldSpec::image(false), json!("data:image/png;base64,iVBORw0KGgo="))]
#[case::timestamp("'2020-01-01'", "value=timestamp", FieldSpec::timestamp(false), json!(1577836800000_i64))]
#[case::text_list("['a','b']", "value=text_list", FieldSpec::list(false, ListValueType::String), json!(["a", "b"]))]
#[case::int_list("[1,2]", "value=int_list", FieldSpec::list(false, ListValueType::Integer), json!([1, 2]))]
#[case::float_list("[1.5,2.5]", "value=float_list", FieldSpec::list(false, ListValueType::Float), json!([1.5, 2.5]))]
#[case::vector("[1,2]", "value=vector:2", FieldSpec::f32_vector(2, false), json!([1.0, 2.0]))]
#[case::matrix("'[1,2,3,4]'", "value=matrix:2", FieldSpec::matrix(false, 2, MatrixValueType::F32), json!([[1.0, 2.0], [3.0, 4.0]]))]
#[case::sparse("'{\"1\":1.5}'", "value=sparse", FieldSpec::f32_sparse_vector(false), json!({"1": 1.5}))]
#[case::keyword("'hello'", "value=keyword", FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Text)), json!("hello"))]
#[case::text_list_keyword("['a','b']", "value=text_list:keyword", FieldSpec::list(false, ListValueType::String).with_index(FieldIndex::keyword(KeywordIndexType::Text)), json!(["a", "b"]))]
#[case::typed_semantic("'hello'", "value=text:semantic", FieldSpec::text(false).with_index(FieldIndex::semantic()), json!("hello"))]
#[case::semantic("'hello'", "value=semantic", FieldSpec::text(false).with_index(FieldIndex::semantic()), json!("hello"))]
#[case::ngram("'hello'", "value=ngram", FieldSpec::text(false).with_index(FieldIndex::ngram()), json!("hello"))]
#[case::vector_cosine("[1.0,2.0]", "value=vector:cosine", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::Cosine)), json!([1.0, 2.0]))]
#[case::vector_euclidean("[1.0,2.0]", "value=vector:2:euclidean", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean)), json!([1.0, 2.0]))]
#[case::vector_dot_product("[1.0,2.0]", "value=vector:2:dot_product", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)), json!([1.0, 2.0]))]
#[case::vector_hamming("[1.0,2.0]", "value=vector:2:hamming", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::Hamming)), json!([1.0, 2.0]))]
#[case::matrix_maxsim("'[1,2,3,4]'", "value=matrix:2:maxsim", FieldSpec::matrix(false, 2, MatrixValueType::F32).with_index(FieldIndex::multi_vector(MultiVectorDistanceMetric::Maxsim, None, None, None)), json!([[1.0, 2.0], [3.0, 4.0]]))]
#[case::sparse_dot_product("'{\"1\":1.5}'", "value=sparse:dot_product", FieldSpec::f32_sparse_vector(false).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)), json!({"1": 1.5}))]
async fn field_choices_import(
    ctx: &Ctx,
    #[case] expression: &str,
    #[case] choice: &str,
    #[case] expected: FieldSpec,
    #[case] expected_value: JsonValue,
) {
    let from = ctx.cli.parquet(
        "choice.parquet",
        &format!("SELECT 'one' AS id, {expression} AS value"),
    );
    let collection = ctx.collection("choice");
    let run = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "-f",
            choice,
            "--dry-run",
        ])
        .success();
    let target = &run.plan().target;
    assert_eq!(target.fields.len(), 1);
    assert!(!target.fields["value"].schema.required);
    assert_eq!(target.fields["value"].schema.data_type, expected.data_type);
    assert_eq!(target.fields["value"].schema.index, expected.index);
    assert_eq!(run.preview()[0]["value"], expected_value);
    let done = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "-f",
            choice,
            "--yes",
        ])
        .success();
    assert_eq!(done.summary().rows, 1);
    let schema = ctx
        .client
        .collections()
        .get(&collection)
        .await
        .unwrap()
        .schema;
    assert!(
        topk::import::plan::Field::new(expected).matches(&schema["value"]),
        "{choice}: {:?}",
        schema["value"]
    );
    assert_eq!(
        ctx.docs(&collection, ["one"]).await[0]["value"],
        expected_value
    );
}

#[rstest_ctx(Ctx)]
async fn keyword_list_reimports(ctx: &Ctx) {
    let from = ctx
        .cli
        .parquet("tags.parquet", "SELECT 'one' AS id, ['a', 'b'] AS tags");
    let collection = ctx.collection("tags");
    let runs: [&[&str]; 2] = [&["-f", "tags=keyword"], &[]];
    for fields in runs {
        let run = ctx
            .cli
            .command()
            .args(["-o", "json", "import", &from, "--to", &collection, "--yes"])
            .args(fields)
            .success();
        assert_eq!(run.summary().rows, 1);
    }
    assert_eq!(
        ctx.docs(&collection, ["one"]).await[0]["tags"],
        json!(["a", "b"])
    );
}

#[rstest_ctx(Ctx)]
async fn matrix_columns_inherit_from_source(ctx: &Ctx) {
    let source = ctx.collection("matrix_source");
    let dest = ctx.collection("matrix_dest");
    let matrix = FieldSpec::matrix(false, 2, MatrixValueType::F32);
    ctx.client
        .collections()
        .create(&source, [("value".into(), matrix.clone())], None)
        .await
        .unwrap();
    ctx.client
        .collection(&source)
        .upsert(vec![doc!(
            "_id" => "one",
            "value" => topk_rs::proto::v1::data::Value::matrix(2, vec![1.0_f32, 2.0, 3.0, 4.0])
        )])
        .await
        .unwrap();
    let done = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &format!("topk://{source}"),
            "--to",
            &dest,
            "-f",
            "value=matrix",
            "--yes",
        ])
        .success();
    assert_eq!(done.summary().rows, 1);
    let schema = ctx.client.collections().get(&dest).await.unwrap().schema;
    assert_eq!(schema["value"].data_type, matrix.data_type);
    assert_eq!(
        ctx.docs(&dest, ["one"]).await[0]["value"],
        json!([[1.0, 2.0], [3.0, 4.0]])
    );
}

#[rstest_ctx(Ctx)]
async fn semantic_image_import(ctx: &Ctx) {
    let image = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD
            .encode(include_bytes!("../../assets/favicon.png"))
    );
    let from = ctx.cli.file(
        "image.jsonl",
        &json!({"id": "one", "value": image}).to_string(),
    );
    let dest = ctx.collection("image_semantic");
    let done = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &dest,
            "-f",
            "value=image:semantic",
            "--yes",
        ])
        .success();
    assert_eq!(done.summary().rows, 1);
    let schema = ctx.client.collections().get(&dest).await.unwrap().schema;
    assert!(topk::import::plan::Field::new(
        FieldSpec::image(false).with_index(FieldIndex::semantic())
    )
    .matches(&schema["value"]));
    assert_eq!(ctx.docs(&dest, ["one"]).await[0]["value"], image);
}

#[rstest_ctx(Ctx)]
async fn native_shapes_survive_index_choices(ctx: &Ctx) {
    let source = ctx.collection("native_source");
    let dest = ctx.collection("native_indexed");
    let f16 = [1.0, 2.0].map(half::f16::from_f32);
    ctx.client
        .collections()
        .create(
            &source,
            [
                ("v".into(), FieldSpec::f16_vector(2, false)),
                ("b".into(), FieldSpec::binary_vector(2, false)),
                (
                    "m".into(),
                    FieldSpec::matrix(false, 2, MatrixValueType::F16),
                ),
                ("s".into(), FieldSpec::f16_sparse_vector(false)),
            ],
            None,
        )
        .await
        .unwrap();
    ctx.client
        .collection(&source)
        .upsert(vec![doc!(
            "_id" => "one",
            "v" => topk_rs::proto::v1::data::Value::list(f16.to_vec()),
            "b" => topk_rs::proto::v1::data::Value::list(vec![1_u8, 2]),
            "m" => topk_rs::proto::v1::data::Value::matrix(2, f16.repeat(2)),
            "s" => topk_rs::proto::v1::data::Value::sparse_vector(vec![1], vec![half::f16::from_f32(1.5)])
        )])
        .await
        .unwrap();
    let uri = format!("topk://{source}");
    ctx.cli
        .command()
        .args([
            "import",
            &uri,
            "--to",
            &dest,
            "-f",
            "v=vector:cosine,b=vector:hamming,m=matrix:maxsim,s=sparse:dot_product",
            "--yes",
        ])
        .success();
    let schema = ctx.client.collections().get(&dest).await.unwrap().schema;
    for (name, expected) in
        [
            (
                "v",
                FieldSpec::f16_vector(2, false)
                    .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine)),
            ),
            (
                "b",
                FieldSpec::binary_vector(2, false)
                    .with_index(FieldIndex::vector(VectorDistanceMetric::Hamming)),
            ),
            (
                "m",
                FieldSpec::matrix(false, 2, MatrixValueType::F16).with_index(
                    FieldIndex::multi_vector(MultiVectorDistanceMetric::Maxsim, None, None, None),
                ),
            ),
            (
                "s",
                FieldSpec::f16_sparse_vector(false)
                    .with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)),
            ),
        ]
    {
        assert!(
            topk::import::plan::Field::new(expected).matches(&schema[name]),
            "{name}: {:?}",
            schema[name]
        );
    }
    assert_eq!(
        ctx.docs(&dest, ["one"]).await,
        [json!({
            "_id": "one", "v": [1.0, 2.0], "b": [1, 2],
            "m": [[1.0, 2.0], [1.0, 2.0]], "s": {"1": 1.5}
        })]
    );
}

#[rstest_ctx(Ctx)]
async fn semantic_preview_warns_in_json(ctx: &Ctx) {
    let from = ctx.cli.parquet(
        "semantic.parquet",
        "SELECT 'one' AS id, repeat('a', 32769) AS body",
    );
    let preview = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "-f",
            "body=semantic",
            "--to",
            "c",
            "--dry-run",
        ])
        .success();
    assert_eq!(preview.plan().target.fields.len(), 1);
    assert!(preview.stderr.contains("will be shortened"), "{preview:?}");
}

#[rstest_ctx(Ctx)]
async fn dry_run_requires_destination(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let with_key = ctx
        .command()
        .envs([("TOPK_API_KEY", "unused")])
        .args(["-o", "json", "import", &from, "--to", "c", "--dry-run"])
        .success();
    let without_credentials = ctx
        .command()
        .env_remove("TOPK_PROJECT_ID")
        .envs([("TOPK_CONFIG_DIR", ctx.scratch())])
        .args(["import", &from, "--to", "c", "--dry-run"])
        .failure(1);
    with_key.plan();
    let destination = format!(
        "# destination: project {}, region {}",
        std::env::var("TOPK_PROJECT_ID").unwrap(),
        std::env::var("TOPK_REGION").unwrap()
    );
    assert!(with_key.stderr.contains(&destination), "{with_key:?}");
    assert!(
        without_credentials.stderr.contains("--project-id"),
        "{without_credentials:?}"
    );
}

#[rstest_ctx(Ctx)]
#[case::yes(&["--yes"])]
#[case::continue_on_error(&["--continue-on-error"])]
#[case::concurrency(&["--concurrency", "2"])]
#[case::batch_bytes(&["--batch-bytes", "1MiB"])]
#[case::batch_bytes_short(&["-b", "1MiB"])]
async fn dry_run_ignores_write_options(ctx: &Ctx, #[case] args: &[&str]) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let state = ctx.scratch().join("runs");
    let run = ctx
        .command()
        .envs([("TOPK_IMPORT_STATE_DIR", &state)])
        .args(["import", &from, "--to", "c", "--dry-run"])
        .args(args)
        .success();
    assert_eq!(run.preview().len(), 3);
    assert!(!state.exists());
}

#[rstest_ctx(Scratch)]
#[case::concurrency_cap(&["missing.parquet", "-c", "1025"], "1025 is not in 1..=1024", 2)]
#[case::zero_batch_bytes(&["x.parquet", "--to", "c", "-b", "0"], "must be at least 1B", 2)]
#[case::preview_flag(&["x.parquet", "--preview"], "unexpected argument '--preview'", 2)]
#[case::resume_partition(&["--resume", "1234abcd", "--partition", "tenant"], "cannot be used with", 2)]
#[case::resume_fields(&["--resume", "1234abcd", "-f", "title=keyword"], "cannot be used with", 2)]
#[case::resume_dry_run(&["--resume", "1234abcd", "--dry-run"], "cannot be used with", 2)]
#[case::resume_path(&["--resume", "../escape"], "run id must be eight lowercase hex digits", 2)]
#[case::incompatible_index(&["x.parquet", "-f", "value=int:semantic"], "incompatible index", 2)]
#[case::unknown_field_choice(&["x.parquet", "-f", "title=unknown"], "unknown field choice", 2)]
#[case::unknown_vector_metric(&["x.parquet", "-f", "value=vector:2:angular"], "unknown vector metric", 2)]
#[case::unknown_matrix_index(&["x.parquet", "-f", "value=matrix:2:cosine"], "unknown matrix index", 2)]
#[case::unknown_sparse_index(&["x.parquet", "-f", "value=sparse:cosine"], "unknown field choice", 2)]
#[case::zero_vector_dimension(&["x.parquet", "-f", "value=vector:0"], "invalid field choice", 2)]
#[case::zero_matrix_columns(&["x.parquet", "-f", "value=matrix:0"], "invalid field choice", 2)]
#[case::extra_vector_shape(&["x.parquet", "-f", "value=vector:2:cosine:other"], "unknown vector metric", 2)]
#[case::empty_vector_metric(&["x.parquet", "-f", "value=vector:2:"], "invalid field choice", 2)]
#[case::no_spec(&["x.parquet", "--spec", "a.toml"], "unexpected argument '--spec'", 2)]
#[case::stdin_source(&["-", "--to", "c", "--id", "id", "--dry-run"], "import does not read stdin", 2)]
#[case::stdin_device(&["/dev/stdin", "--to", "c", "--dry-run"], "import does not read stdin", 2)]
#[case::missing_destination(&["x.parquet", "--dry-run"], "--to <COLLECTION>", 2)]
#[case::invalid_destination(&["x.parquet", "--to", "a/b"], "collection names start with a letter or digit", 2)]
#[case::object_argument(&["postgres://host/db#orders", "orders", "--to", "orders"], "put the table, collection or index in its URI", 2)]
#[case::old_syntax(&["postgres://host/db", "orders", "--to", "orders"], "postgres source requires #<table>", 2)]
#[case::bare_field(&["x.parquet", "--to", "c", "-f", "title"], "-f takes <field>=<type or index>", 2)]
#[case::bucket_prefix(&["s3://bucket/events/", "--to", "c"], "use 's3://bucket/events/*.parquet'", 2)]
fn rejected_arguments(
    ctx: &Scratch,
    #[case] args: &[&str],
    #[case] fragment: &str,
    #[case] code: i32,
) {
    let run = ctx
        .command()
        .args(["-o", "json"])
        .args(["import"])
        .args(args)
        .failure(code);
    assert!(run.stderr.contains(fragment), "{run:?}");
}

#[rstest_ctx(Scratch)]
async fn resume_rejects_state_without_project(ctx: &Scratch) {
    ctx.file(
        "state/1234abcd.toml",
        "source = \"\"\nregion = \"dev\"\n[plan]\ncollection = \"c\"\n[plan.target]\nfrom = { local = \"a.parquet\" }\nid = \"_id\"\nexclude = []\n[plan.target.fields]\n[read]\n[write]\ncontinue_on_error = false\nconcurrency = 16\nbatch_bytes = 1048576\n",
    );
    let run = ctx
        .command()
        .args(["-o", "json", "import", "--resume", "1234abcd"])
        .failure(1);
    assert!(run.stderr.contains("missing field"), "{run:?}");
    assert!(run.stderr.contains("project_id"), "{run:?}");
}

#[rstest_ctx(Ctx)]
async fn partition_column_previews(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let preview = ctx
        .command()
        .args([
            "import",
            &from,
            "--to",
            "c",
            "--partition",
            "author",
            "--dry-run",
        ])
        .success();
    assert!(preview.stderr.contains("# partition Lee"), "{preview:?}");
}

#[rstest_ctx(Scratch)]
async fn missing_id_rejected(ctx: &Scratch) {
    let from = ctx.parquet("no-id.parquet", "SELECT 'x' AS a, 'y' AS b");
    let preview = ctx
        .command()
        .args(["import", &from, "--to", "c", "--dry-run"])
        .failure(1);
    let done = ctx
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            "c",
            "--yes",
            "--project-id",
            "p1",
        ])
        .failure(1);
    assert!(preview.stdout.is_empty(), "{preview:?}");
    assert!(preview.stderr.contains("--id"), "{preview:?}");
    assert!(preview.stderr.contains("available: a, b"), "{preview:?}");
    assert!(done.stderr.contains("--id"), "{done:?}");
}

#[rstest_ctx(Ctx)]
async fn a_cli_filter_reaches_the_target(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let run = ctx
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            "c",
            "--filter",
            "published_year > 1950",
            "--dry-run",
        ])
        .success();
    assert_eq!(
        run.plan().target.filter.as_deref(),
        Some("published_year > 1950")
    );
    assert_eq!(
        run.preview()
            .iter()
            .map(|doc| doc["_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["mockingbird"]
    );
}

#[rstest_ctx(Scratch)]
async fn empty_region_is_rejected(ctx: &Scratch) {
    let from = ctx.seed_parquet("books", books());
    let run = ctx
        .command()
        .envs([("TOPK_REGION", "")])
        .args(["-o", "json", "import", &from, "--to", "c", "--yes"])
        .failure(2);
    assert!(
        run.stderr
            .contains("a value is required for '--region <REGION>'"),
        "{run:?}"
    );
}

#[rstest_ctx(Ctx)]
async fn yes_required_without_tty(ctx: &Ctx) {
    let from = ctx.cli.seed_parquet("books", books());
    let collection = ctx.collection("confirm");
    let state = ctx.cli.scratch().join("unconfirmed");
    let run = ctx
        .cli
        .command()
        .envs([("TOPK_IMPORT_STATE_DIR", &state)])
        .args(["-o", "json", "import", &from, "--to", &collection])
        .failure(1);
    assert!(run.stderr.contains("pass --yes"), "{run:?}");
    assert_eq!(std::fs::read_dir(&state).unwrap().count(), 0);
    assert!(!ctx.exists(&collection).await);
}

#[rstest_ctx(Ctx)]
#[case::missing_field(&["-f", "ghost=keyword"], "field \"ghost\" was not found")]
#[case::duplicate_field(&["-f", "value=int,value=int"], "field \"value\" selected twice")]
#[case::missing_rename_source(&["--rename", "nonexistent_col=ghost"], "source field \"nonexistent_col\" was not found")]
#[case::rename_collision(&["--rename", "title=value"], "field \"value\" already exists")]
#[case::inferred_index_mismatch(&["-f", "value=semantic"], "incompatible index for field type")]
#[case::id_as_field(&["-f", "id=text"], "\"id\" is the document id")]
#[case::rename_twice(&["--rename", "title=a", "--rename", "title=b"], "column \"title\" is renamed twice")]
#[case::excluded_and_set(&["--exclude", "title", "-f", "title=keyword"], "both excluded and set with -f")]
#[case::missing_exclude(&["--exclude", "ghost"], "excluded field \"ghost\" was not found")]
#[case::missing_id(&["--id", "nope"], "id column \"nope\" is not in")]
#[case::missing_partition(&["--partition", "nope"], "partition column \"nope\" is not in")]
async fn invalid_fields_fail_before_creating(
    ctx: &Ctx,
    #[case] args: &[&str],
    #[case] expected: &str,
) {
    let from = ctx.cli.parquet(
        "bad-field.parquet",
        "SELECT 1 AS id, 'dune' AS title, 42 AS value",
    );
    let collection = ctx.collection("bad-field");
    let done = ctx
        .cli
        .command()
        .args(["-o", "json", "import", &from, "--to", &collection])
        .args(args)
        .args(["--yes"])
        .failure(1);
    assert!(done.stderr.contains(expected), "{done:?}");
    assert!(!ctx.exists(&collection).await);
}

#[rstest_ctx(Ctx)]
async fn reimport_replaces_fields(ctx: &Ctx) {
    let from = ctx.cli.seed_parquet("books", books());
    let collection = ctx.collection("reimport");
    let first = ctx
        .cli
        .command()
        .args(["-o", "json", "import", &from, "--to", &collection, "--yes"])
        .success();
    let narrowed = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "--exclude",
            "author,published_year,rating,in_print",
            "--yes",
        ])
        .success();
    let docs = ctx.docs(&collection, BOOK_IDS).await;
    assert_eq!(docs.len(), 3);
    assert!(docs.iter().all(|doc| doc.as_object().unwrap().len() == 2));
    let full = ctx
        .cli
        .command()
        .args(["-o", "json", "import", &from, "--to", &collection, "--yes"])
        .success();
    let destination = format!(
        "# destination: project {}, region {}",
        std::env::var("TOPK_PROJECT_ID").unwrap(),
        std::env::var("TOPK_REGION").unwrap()
    );
    assert!(first.stderr.contains(&destination), "{first:?}");
    assert_eq!(first.summary().rows, 3);
    assert_eq!(narrowed.summary().rows, 3);
    assert_eq!(full.summary().rows, 3);
    let expected: Vec<_> = books().iter().map(|doc| json!(json(doc))).collect();
    assert_eq!(ctx.docs(&collection, BOOK_IDS).await, expected);
}

#[rstest_ctx(Ctx)]
#[case::text("text")]
#[case::json("json")]
async fn skipped_rows_are_reported(ctx: &Ctx, #[case] format: &str) {
    let from = ctx.cli.seed_parquet(
        "bad-row",
        vec![
            doc!("id" => 1, "n" => "5"),
            doc!("id" => 2, "n" => "no"),
            doc!("id" => 3, "n" => "7"),
            doc!("id" => 4, "n" => "no"),
            doc!("id" => 5, "n" => "no"),
            doc!("id" => 6, "n" => "no"),
            doc!("id" => 7, "n" => "no"),
        ],
    );
    let collection = ctx.collection("summary");
    let output = ctx
        .cli
        .command()
        .args([
            "import",
            &from,
            "--to",
            &collection,
            "-f",
            "n=int",
            "--yes",
            "--continue-on-error",
            "-o",
            format,
        ])
        .success();
    assert!(
        output
            .stderr
            .contains("skipped row 2: doc \"2\" field \"n\""),
        "{output:?}"
    );
    assert_eq!(
        output
            .stderr
            .lines()
            .filter(|line| line.starts_with("skipped row "))
            .count(),
        3,
        "{output:?}"
    );
    match format {
        "text" => {
            assert!(output.stdout.is_empty(), "{output:?}");
            assert!(
                output.stderr.contains("2 rows written, 5 failed"),
                "{output:?}"
            );
        }
        "json" => {
            let outcome = output.summary();
            assert_eq!((outcome.rows, outcome.failed), (2, 5));
            assert!(!output.stderr.contains("rows written"), "{output:?}");
        }
        _ => unreachable!(),
    }
    assert!(
        output
            .stderr
            .contains("more rows skipped; every skipped row is listed in "),
        "{output:?}"
    );
    let report = output
        .stderr
        .lines()
        .find_map(|line| line.strip_prefix("5 skipped rows are listed in "))
        .unwrap_or_else(|| panic!("{output:?}"));
    let rejected: Vec<JsonValue> = std::fs::read_to_string(report)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        rejected
            .iter()
            .map(|row| (row["row"].as_u64().unwrap(), row["id"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        [(2, "2"), (4, "4"), (5, "5"), (6, "6"), (7, "7")]
    );
    assert!(
        rejected.iter().all(|row| row["field"] == "n"),
        "{rejected:?}"
    );
    assert_eq!(
        ctx.docs(&collection, [1, 2, 3]).await,
        [json!({"_id": "1", "n": 5}), json!({"_id": "3", "n": 7})]
    );
}

#[rstest_ctx(Ctx)]
async fn preview_elides_large_values(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.parquet(
        "data.parquet",
        "SELECT 1 AS id, repeat('é', 300) AS title, range(10) AS numbers",
    );
    let output = ctx
        .command()
        .args(["import", &from, "--to", "c", "--dry-run"])
        .success();
    assert!(output.stdout.contains("title: text"), "{output:?}");
    let sample = output
        .stderr
        .lines()
        .find(|line| line.starts_with('{'))
        .unwrap();
    assert_eq!(sample.chars().count(), 241, "{output:?}");
    assert!(sample.ends_with('…'), "{output:?}");
    assert!(sample.contains('é'), "{output:?}");
    assert!(!output.stdout.contains('…'), "{output:?}");
}

#[rstest_ctx(Ctx)]
async fn successful_text_summary(ctx: &Ctx) {
    let from = ctx.cli.seed_parquet("books", books());
    let collection = ctx.collection("text-summary");
    let output = ctx
        .cli
        .command()
        .args(["import", &from, "--to", &collection, "--yes"])
        .success();
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(
        output.stderr.matches("3 rows written, 0 failed").count(),
        1,
        "{output:?}"
    );
}

#[rstest_ctx(Ctx)]
async fn failure_prints_resume_command(ctx: &Ctx) {
    let from = ctx
        .cli
        .seed_parquet("bad-row", vec![doc!("id" => 1, "n" => "not-a-number")]);
    let collection = ctx.collection("resume-hint");
    let output = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "-f",
            "n=int",
            "--yes",
        ])
        .failure(1);
    assert!(
        output.stderr.contains(&format!(
            "topk import --resume {} --continue-on-error",
            output.id()
        )),
        "{output:?}"
    );
    assert!(ctx.cli.state_path(output.id()).exists());
    let resumed = ctx
        .cli
        .command()
        .args(["import", "--resume", &output.id().to_string()])
        .args(["--continue-on-error"])
        .success();
    assert!(
        resumed.stderr.contains("0 rows written, 1 failed")
            && resumed.stderr.contains("1 skipped rows are listed in "),
        "{resumed:?}"
    );
}

#[rstest_ctx(Ctx)]
async fn resume_from_another_directory(ctx: &Ctx) {
    ctx.cli.seed_parquet(
        "data/visits",
        vec![
            doc!("id" => 1, "n" => "1"),
            doc!("id" => 2, "n" => "not-a-number"),
            doc!("id" => 3, "n" => "3"),
        ],
    );
    let collection = ctx.collection("resume-elsewhere");
    let stopped = ctx
        .cli
        .command()
        .current_dir(ctx.cli.path("data"))
        .args([
            "-o",
            "json",
            "import",
            "visits.parquet",
            "--to",
            &collection,
        ])
        .args(["-f", "n=int", "--yes"])
        .failure(1);
    let resumed = ctx
        .cli
        .command()
        .current_dir(ctx.cli.scratch())
        .args([
            "-o",
            "json",
            "import",
            "--resume",
            stopped.id(),
            "--continue-on-error",
        ])
        .success();
    assert!(stopped.stderr.contains("← visits.parquet"), "{stopped:?}");
    assert!(
        resumed.stderr.contains("← data/visits.parquet"),
        "{resumed:?}"
    );
    assert_eq!(resumed.summary().failed, 1, "{resumed:?}");
    assert_eq!(ctx.docs(&collection, ["1", "3"]).await.len(), 2);
}

#[rstest_ctx(Ctx)]
#[case::default_cap(None, 5)]
#[case::zero_limit(Some("0"), 0)]
#[case::lower_limit(Some("2"), 2)]
#[case::higher_limit(Some("10"), 5)]
async fn preview_limits(ctx: &Ctx, #[case] limit: Option<&str>, #[case] expected: usize) {
    let from = ctx.cli.seed_parquet("rows", rows(1, 20));
    let collection = ctx.collection("preview");
    let preview = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "--dry-run",
        ])
        .args(limit.into_iter().flat_map(|limit| ["--limit", limit]))
        .success();
    assert_eq!(preview.plan().target.id, "_id");
    if let Some(limit) = limit {
        assert_eq!(preview.plan().target.limit, Some(limit.parse().unwrap()));
    }
    assert_eq!(preview.preview().len(), expected);
    assert!(
        preview.stderr.contains(&format!("# → {collection}")),
        "{preview:?}"
    );
    assert!(!ctx.exists(&collection).await);
}

#[rstest_ctx(Ctx)]
async fn imports_filtered_documents(ctx: &Ctx) {
    let from = ctx.cli.seed_parquet("books", books());
    let collection = ctx.collection("filtered");
    let done = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "--filter",
            "published_year > 1950",
            "--yes",
        ])
        .success();
    assert_eq!(done.summary().rows, 1);
    let expected = json!({"_id": "mockingbird", "title": "To Kill a Mockingbird", "author": "Lee",
        "published_year": 1960, "rating": 4.3, "in_print": true});
    assert_eq!(
        ctx.docs(
            &collection,
            ["mockingbird", "nineteen_eighty_four", "pride"]
        )
        .await,
        [expected]
    );
}

#[rstest_ctx(Ctx)]
async fn copies_collection(ctx: &Ctx) {
    let from = ctx.cli.seed_parquet("books", books());
    let source = ctx.collection("original");
    let dest = ctx.collection("copy");
    ctx.cli
        .command()
        .args(["import", &from, "--to", &source, "--yes"])
        .success();
    ctx.cli
        .command()
        .args([
            "import",
            &format!("topk://{source}"),
            "--to",
            &dest,
            "--yes",
        ])
        .success();
    let expected: Vec<_> = books().iter().map(|doc| json!(json(doc))).collect();
    assert_eq!(ctx.docs(&dest, BOOK_IDS).await, expected);
}

#[rstest_ctx(Ctx)]
async fn existing_collection_keeps_its_schema(ctx: &Ctx) {
    let collection = ctx.collection("declared");
    let embedding = FieldSpec::f32_vector(3, false)
        .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine));
    ctx.client
        .collections()
        .create(&collection, [("embedding".into(), embedding)], None)
        .await
        .unwrap();
    let from = ctx.cli.parquet(
        "declared.parquet",
        "SELECT 'one' AS id, 'Dune' AS title, 1965 AS year, [1.0, 0.0, 0.0]::FLOAT[] AS embedding",
    );
    let done = ctx
        .cli
        .command()
        .args(["-o", "json", "import", &from, "--to", &collection, "--yes"])
        .success();
    assert_eq!(done.summary().rows, 1);
    let schema = ctx
        .client
        .collections()
        .get(&collection)
        .await
        .unwrap()
        .schema;
    assert_eq!(schema.keys().collect::<Vec<_>>(), ["embedding"]);
    assert_eq!(
        ctx.docs(&collection, ["one"]).await,
        [json!({"_id": "one", "title": "Dune", "year": 1965, "embedding": [1.0, 0.0, 0.0]})]
    );
    let indexed = ctx
        .cli
        .command()
        .args(["import", &from, "--to", &collection])
        .args(["-f", "title=keyword", "--yes"])
        .failure(1);
    assert!(
        indexed
            .stderr
            .contains("title: an index needs the field declared"),
        "{indexed:?}"
    );
}

#[rstest_ctx(Ctx)]
async fn declared_required_field_must_come_from_the_source(ctx: &Ctx) {
    let collection = ctx.collection("required");
    ctx.client
        .collections()
        .create(
            &collection,
            [
                ("title".into(), FieldSpec::text(false)),
                ("body".into(), FieldSpec::text(true)),
            ],
            None,
        )
        .await
        .unwrap();
    let from = ctx
        .cli
        .parquet("titles.parquet", "SELECT 'one' AS id, 'Dune' AS title");
    let run = ctx
        .cli
        .command()
        .args(["import", &from, "--to", &collection, "--yes"])
        .failure(1);
    assert!(
        run.stderr
            .contains("body: the collection requires it, and this import leaves it out"),
        "{run:?}"
    );
    assert!(ctx.docs(&collection, ["one"]).await.is_empty());
}

#[rstest_ctx(Ctx)]
async fn failed_first_write_leaves_no_collection(ctx: &Ctx) {
    let collection = ctx.collection("lazy-create");
    let from = ctx
        .cli
        .parquet("titles.parquet", "SELECT 'one' AS id, 'hello' AS title");
    let failed = ctx
        .cli
        .command()
        .args(["import", &from, "--to", &collection])
        .args(["-f", "title=vector:3:cosine", "--yes"])
        .failure(1);
    assert!(failed.stderr.contains("title"), "{failed:?}");
    assert!(!ctx.exists(&collection).await);
    ctx.cli
        .command()
        .args(["import", &from, "--to", &collection, "--yes"])
        .success();
    assert_eq!(
        ctx.docs(&collection, ["one"]).await,
        [json!({"_id": "one", "title": "hello"})]
    );
}

#[rstest_ctx(Ctx)]
async fn reader_failure_cannot_be_skipped(ctx: &Ctx) {
    let from = ctx.cli.parquet(
        "late.parquet",
        "SELECT i AS id, CASE WHEN i = 25000 THEN 'n/a' ELSE i::VARCHAR END AS n FROM range(25001) t(i)",
    );
    let collection = ctx.collection("late-value");
    let skipped = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "--filter",
            "n::BIGINT >= 0",
            "--yes",
            "--continue-on-error",
        ])
        .failure(1);
    assert!(skipped.stderr.contains("Conversion Error"), "{skipped:?}");
}

#[rstest_ctx(Ctx)]
#[case::keeps(
    vec![("title", FieldSpec::text(false))],
    None,
    &[],
    json!({"_id": "one", "title": "Dune", "year": 1965, "draft": true}),
    vec![("title", FieldSpec::text(false))]
)]
#[case::selects(
    vec![("title", FieldSpec::text(false))],
    None,
    &["--exclude", "draft", "-f", "year=int"],
    json!({"_id": "one", "title": "Dune", "year": 1965}),
    vec![("title", FieldSpec::text(false)), ("year", FieldSpec::integer(false))]
)]
#[case::required_destination(
    vec![("title", FieldSpec::text(false))],
    Some(vec![("year", FieldSpec::integer(true))]),
    &[],
    json!({"_id": "one", "title": "Dune", "year": 1965, "draft": true}),
    vec![("year", FieldSpec::integer(true))]
)]
#[case::declared_type_converts(
    vec![("title", FieldSpec::text(false))],
    Some(vec![("year", FieldSpec::float(false))]),
    &[],
    json!({"_id": "one", "title": "Dune", "year": 1965.0, "draft": true}),
    vec![("year", FieldSpec::float(false))]
)]
#[case::renames_undeclared(
    vec![("title", FieldSpec::text(false))],
    None,
    &["--rename", "year=released", "-f", "released=int"],
    json!({"_id": "one", "title": "Dune", "released": 1965, "draft": true}),
    vec![("title", FieldSpec::text(false)), ("released", FieldSpec::integer(false))]
)]
#[case::index_on_undeclared(
    vec![],
    None,
    &["-f", "title=keyword"],
    json!({"_id": "one", "title": "Dune", "year": 1965, "draft": true}),
    vec![("title", FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Text)))]
)]
#[case::no_declared_fields(
    vec![],
    None,
    &[],
    json!({"_id": "one", "title": "Dune", "year": 1965, "draft": true}),
    vec![]
)]
async fn copy_undeclared_fields(
    ctx: &Ctx,
    #[case] source_schema: Vec<(&str, FieldSpec)>,
    #[case] destination: Option<Vec<(&str, FieldSpec)>>,
    #[case] args: &[&str],
    #[case] expected: JsonValue,
    #[case] expected_schema: Vec<(&str, FieldSpec)>,
) {
    let schema = |fields: Vec<(&str, FieldSpec)>| -> HashMap<String, FieldSpec> {
        fields
            .into_iter()
            .map(|(name, spec)| (name.into(), spec))
            .collect()
    };
    let source = ctx.collection("undeclared-source");
    let dest = ctx.collection("undeclared-copy");
    ctx.client
        .collections()
        .create(&source, schema(source_schema), None)
        .await
        .unwrap();
    if let Some(fields) = destination {
        ctx.client
            .collections()
            .create(&dest, schema(fields), None)
            .await
            .unwrap();
    }
    ctx.client
        .collection(&source)
        .upsert(vec![
            doc!("_id" => "one", "title" => "Dune", "year" => 1965_i64, "draft" => true),
        ])
        .await
        .unwrap();
    ctx.cli
        .command()
        .args(["-o", "json", "import", &format!("topk://{source}")])
        .args(["--to", &dest, "--yes"])
        .args(args)
        .success();
    assert_eq!(ctx.docs(&dest, ["one"]).await, [expected]);
    assert_eq!(
        ctx.client.collections().get(&dest).await.unwrap().schema,
        schema(expected_schema)
    );
}

#[rstest_ctx(Ctx)]
async fn copy_onto_itself_is_refused(ctx: &Ctx) {
    let from = ctx.cli.seed_parquet("books", books());
    let collection = ctx.collection("self-copy");
    ctx.cli
        .command()
        .args(["import", &from, "--to", &collection, "--yes"])
        .success();
    let run = ctx
        .cli
        .command()
        .args([
            "import",
            &format!("topk://{collection}"),
            "--to",
            &collection,
        ])
        .args(["--yes"])
        .failure(1);
    assert!(run.stderr.contains("is the destination"), "{run:?}");
}

#[rstest_ctx(Scratch)]
fn folder_source_names_the_glob(ctx: &Scratch) {
    ctx.parquet("events/a.parquet", "SELECT 1 AS id, 'x' AS title");
    ctx.file("events/b.jsonl", r#"{"id":2,"title":"y"}"#);
    let dir = ctx.path("events");
    let run = ctx
        .command()
        .args(["import", dir.to_str().unwrap(), "--to", "c", "--dry-run"])
        .failure(2);
    let dir = dir.display();
    assert!(
        run.stderr
            .contains(&format!("use '{dir}/*.jsonl' or '{dir}/*.parquet'")),
        "{run:?}"
    );
}

#[rstest_ctx(Scratch)]
#[case::working_directory("data/books.parquet")]
#[case::home("~/data/books.parquet")]
fn braced_folder_above_source_is_literal(ctx: &Scratch, #[case] from: &str) {
    ctx.parquet("{me}/data/books.parquet", "SELECT 'x' AS a, 'y' AS b");
    let run = ctx
        .command()
        .current_dir(ctx.path("{me}"))
        .envs([("HOME", ctx.path("{me}"))])
        .args(["import", from, "--to", "c", "--dry-run"])
        .failure(1);
    assert!(run.stderr.contains("available: a, b —"), "{run:?}");
}

#[rstest_ctx(Ctx)]
async fn copy_skips_rows_without_required_field(ctx: &Ctx) {
    let source = ctx.collection("required-rows");
    let dest = ctx.collection("required-rows-copy");
    ctx.client
        .collections()
        .create(&source, HashMap::new(), None)
        .await
        .unwrap();
    ctx.client
        .collections()
        .create(&dest, [("year".into(), FieldSpec::integer(true))], None)
        .await
        .unwrap();
    ctx.client
        .collection(&source)
        .upsert(vec![
            doc!("_id" => "one", "year" => 1965_i64),
            doc!("_id" => "two", "title" => "Emma"),
        ])
        .await
        .unwrap();
    let done = ctx
        .cli
        .command()
        .args(["-o", "json", "import", &format!("topk://{source}")])
        .args(["--to", &dest, "--continue-on-error", "--yes"])
        .success();
    assert_eq!((done.summary().rows, done.summary().failed), (1, 1));
    assert_eq!(
        ctx.docs(&dest, ["one", "two"]).await,
        [json!({"_id": "one", "year": 1965})]
    );
}

#[rstest_ctx(Ctx)]
async fn excluding_every_field_imports_ids(ctx: &Ctx) {
    let from = ctx
        .cli
        .parquet("ids.parquet", "SELECT 'one' AS id, 'x' AS title");
    let dest = ctx.collection("ids-only");
    ctx.cli
        .command()
        .args([
            "import",
            &from,
            "--to",
            &dest,
            "--exclude",
            "title",
            "--yes",
        ])
        .success();
    assert_eq!(ctx.docs(&dest, ["one"]).await, [json!({"_id": "one"})]);
    assert!(ctx
        .client
        .collections()
        .get(&dest)
        .await
        .unwrap()
        .schema
        .is_empty());
}

#[rstest_ctx(Ctx)]
async fn rare_partition_checkpoint_advances(ctx: &Ctx) {
    let from = ctx.cli.parquet(
        "skew.parquet",
        "SELECT CASE WHEN i = 3999 THEN NULL ELSE i END AS id, CASE WHEN i = 0 THEN 'rare' ELSE 'hot' END AS tenant, repeat('x', 400) AS pad FROM range(0, 4000) t(i)",
    );
    let collection = ctx.collection("skew");
    let stopped = ctx
        .cli
        .command()
        .args([
            "import",
            &from,
            "--to",
            &collection,
            "--partition",
            "tenant",
        ])
        .args(["--batch-bytes", "16KB", "-c", "1", "--yes"])
        .failure(1);
    let state: topk::import::state::State =
        toml::from_str(&std::fs::read_to_string(ctx.cli.state_path(stopped.id())).unwrap())
            .unwrap();
    let consumed = match &state.progress {
        Some(topk::import::state::Progress::After(checkpoint)) => checkpoint.consumed,
        _ => 0,
    };
    assert!(consumed > 0, "no checkpoint saved");
}

#[rstest_ctx(Ctx)]
async fn untyped_columns_fail_the_plan(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.file(
        "lists.jsonl",
        r#"{"id":"one","title":"book","flags":[true],"messages":[{"role":"user"}],"ts":["2024-01-01T00:00:00Z"]}"#,
    );
    let refused = ctx
        .command()
        .args(["import", &from, "--id", "id", "--to", "c", "--dry-run"])
        .failure(1);
    assert!(refused.stdout.is_empty(), "{refused:?}");
    assert!(
        refused
            .stderr
            .contains("flags, messages, ts have no TopK type"),
        "{refused:?}"
    );
    assert!(
        refused.stderr.contains("--exclude flags,messages,ts"),
        "{refused:?}"
    );
    let run = ctx
        .command()
        .args(["import", &from, "--id", "id", "--to", "c"])
        .args(["--exclude", "flags,messages,ts", "--dry-run"])
        .success();
    assert!(run.stdout.contains("title: text"), "{run:?}");
    assert!(run.stderr.contains("book"), "{run:?}");
}

#[rstest_ctx(Scratch)]
async fn empty_database_names_source(ctx: &Scratch) {
    let path = ctx.file("empty.db", "");
    let url = format!("sqlite:{path}#missing");
    let run = ctx
        .command()
        .args(["import", &url, "--to", "c", "--dry-run"])
        .failure(1);
    assert!(run.stderr.contains(&path), "{run:?}");
    assert!(run.stderr.contains("was not found"), "{run:?}");
}

#[rstest_ctx(Ctx)]
#[case::all_rows(None, 27)]
#[case::limited_rows(Some(24), 24)]
async fn partition_failure_resumes_saved_prefix(
    ctx: &Ctx,
    #[case] limit: Option<u64>,
    #[case] consumed: u64,
) {
    for (file, lo, hi) in [("a", 0, 11), ("b", 11, 27)] {
        ctx.cli.parquet(
            &format!("{file}.parquet"),
            &format!("SELECT CASE WHEN i = 23 THEN NULL ELSE i END AS id, 'tenant_' || (i % 2) AS tenant, repeat('x', 400) AS pad FROM range({lo}, {hi}) t(i)"),
        );
    }
    let from = format!("{}/*.parquet", ctx.cli.scratch().display());
    let collection = ctx.collection("partition-resume");
    let mut command = ctx.cli.command().args([
        "import",
        &from,
        "--to",
        &collection,
        "--partition",
        "tenant",
        "--batch-bytes",
        "1B",
        "-c",
        "1",
        "--readers",
        "1",
        "--yes",
    ]);
    if let Some(limit) = limit {
        command = command.args(["--limit", &limit.to_string()]);
    }
    let stopped = command.failure(1);
    assert!(stopped.stderr.contains("id is null"), "{stopped:?}");
    let state: topk::import::state::State =
        toml::from_str(&std::fs::read_to_string(ctx.cli.state_path(stopped.id())).unwrap())
            .unwrap();
    assert!(matches!(
        &state.progress,
        Some(topk::import::state::Progress::After(checkpoint)) if checkpoint.consumed == 11
    ));
    let resumed = ctx
        .cli
        .command()
        .args([
            "import",
            "--resume",
            stopped.id(),
            "--batch-bytes",
            "1B",
            "-c",
            "2",
            "--readers",
            "1",
            "--continue-on-error",
            "-o",
            "json",
        ])
        .success();
    let summary = resumed.summary();
    assert_eq!(
        (summary.rows, summary.failed, summary.resumed_after),
        ((consumed - 12) as usize, 1, 11)
    );
    for partition in 0..2 {
        let ids: Vec<_> = (0..consumed)
            .filter(|id| *id != 23 && id % 2 == partition)
            .map(|id| id.to_string())
            .collect();
        let documents = ctx
            .client
            .collection(&collection)
            .partition(format!("tenant_{partition}"))
            .get(
                ids.clone(),
                None,
                None,
                Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong),
            )
            .await
            .unwrap();
        assert_eq!(documents.len(), ids.len());
        for id in ids {
            assert_eq!(
                documents[&id]["pad"].as_string(),
                Some("x".repeat(400).as_str())
            );
        }
        assert_eq!(
            ctx.client
                .collection(&collection)
                .partition(format!("tenant_{partition}"))
                .count(
                    None,
                    Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong)
                )
                .await
                .unwrap(),
            documents.len() as u64
        );
    }
    assert!(!ctx.cli.state_path(stopped.id()).exists());
}

#[rstest_ctx(Ctx)]
#[case::one_reader("1")]
#[case::three_readers("3")]
async fn many_files_import_every_row(ctx: &Ctx, #[case] readers: &str) {
    for file in 0..6 {
        ctx.cli.parquet(
            &format!("part-{file}.parquet"),
            &format!("SELECT ({file} * 1000 + i)::VARCHAR AS id, repeat('x', 400) AS pad FROM range(300) t(i)"),
        );
    }
    let from = format!("{}/*.parquet", ctx.cli.scratch().display());
    let collection = ctx.collection("many-files");
    let summary = ctx
        .cli
        .command()
        .args([
            "import",
            &from,
            "--to",
            &collection,
            "--readers",
            readers,
            "--chunk-bytes",
            "8KiB",
            "--batch-bytes",
            "16KiB",
            "--yes",
            "-o",
            "json",
        ])
        .success()
        .summary();
    assert_eq!((summary.rows, summary.failed), (1800, 0));
    let strong = Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong);
    let collection = ctx.client.collection(&collection);
    assert_eq!(collection.count(None, strong).await.unwrap(), 1800);
    let ids: Vec<String> = (0..6)
        .flat_map(|file| [file * 1000, file * 1000 + 299])
        .map(|id| id.to_string())
        .collect();
    let documents = collection
        .get(ids.clone(), None, None, strong)
        .await
        .unwrap();
    assert_eq!(documents.len(), ids.len());
}

#[rstest_ctx(Ctx)]
async fn shards_import_every_file_once(ctx: &Ctx) {
    for file in 0..9 {
        ctx.cli.parquet(
            &format!("part-{file}.parquet"),
            &format!("SELECT ({file} * 1000 + i)::VARCHAR AS id, 't' AS text FROM range(100) t(i)"),
        );
    }
    let from = format!("{}/*.parquet", ctx.cli.scratch().display());
    let collection = ctx.collection("shards");
    let mut rows = Vec::new();
    for shard in ["1/2", "2/2"] {
        let summary = ctx
            .cli
            .command()
            .args([
                "import",
                &from,
                "--to",
                &collection,
                "--shard",
                shard,
                "--yes",
                "-o",
                "json",
            ])
            .success()
            .summary();
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.rows % 100, 0, "a shard reads whole files");
        rows.push(summary.rows);
    }
    assert!(rows.iter().all(|&rows| rows > 0), "{rows:?}");
    assert_eq!(rows.iter().sum::<usize>(), 900);
    let strong = Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong);
    assert_eq!(
        ctx.client
            .collection(&collection)
            .count(None, strong)
            .await
            .unwrap(),
        900
    );
}

#[rstest_ctx(Scratch)]
async fn shard_needs_a_file_source(ctx: &Scratch) {
    let run = ctx
        .command()
        .args([
            "import",
            "postgres://u:p@localhost:1/db#t",
            "--to",
            "c",
            "--shard",
            "1/2",
            "--yes",
        ])
        .failure(1);
    assert!(
        run.stderr.contains("--shard splits a glob's files"),
        "{run:?}"
    );
}

#[rstest_ctx(Ctx)]
async fn stopped_files_resume_every_row(ctx: &Ctx) {
    for file in 0..4 {
        ctx.cli.parquet(
            &format!("part-{file}.parquet"),
            &format!("SELECT CASE WHEN {file} = 3 AND i = 180 THEN NULL ELSE ({file} * 1000 + i)::VARCHAR END AS id, repeat('x', 400) AS pad FROM range(200) t(i)"),
        );
    }
    let from = format!("{}/*.parquet", ctx.cli.scratch().display());
    let collection = ctx.collection("files-resume");
    let sizes = [
        "--readers",
        "3",
        "--chunk-bytes",
        "8KiB",
        "--batch-bytes",
        "16KiB",
        "-c",
        "1",
    ];
    let stopped = ctx
        .cli
        .command()
        .args(["import", &from, "--to", &collection, "--yes"])
        .args(sizes)
        .failure(1);
    assert!(stopped.stderr.contains("id is null"), "{stopped:?}");
    let summary = ctx
        .cli
        .command()
        .args([
            "import",
            "--resume",
            stopped.id(),
            "--continue-on-error",
            "-o",
            "json",
        ])
        .args(sizes)
        .success()
        .summary();
    assert!(
        summary.resumed_after > 0,
        "nothing was saved before the stop"
    );
    assert_eq!(
        (
            summary.failed,
            summary.resumed_after + summary.rows as u64 + 1
        ),
        (1, 800)
    );
    let strong = Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong);
    let collection = ctx.client.collection(&collection);
    assert_eq!(collection.count(None, strong).await.unwrap(), 799);
    let ids: Vec<String> = (0..4)
        .flat_map(|file| [file * 1000, file * 1000 + 179, file * 1000 + 199])
        .map(|id| id.to_string())
        .collect();
    let documents = collection
        .get(ids.clone(), None, None, strong)
        .await
        .unwrap();
    assert_eq!(documents.len(), ids.len());
    assert!(!ctx.cli.state_path(stopped.id()).exists());
}

#[rstest_ctx(Ctx)]
async fn missing_file_is_readable(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let path = ctx.scratch().join("missing.parquet");
    let path = path.to_str().unwrap();
    let output = ctx
        .command()
        .args(["import", path, "--to", "c", "--dry-run"])
        .failure(1);
    assert!(
        output
            .stderr
            .contains(&format!("error: file not found: {path}\n")),
        "{}",
        output.stderr
    );
    assert!(!output.stderr.contains("SELECT"), "{}", output.stderr);
}

#[rstest_ctx(Ctx)]
#[case::bare(&[])]
#[case::explicit(&["-f", "embedding=vector:cosine"])]
async fn dry_run_uses_destination(ctx: &Ctx, #[case] field: &[&str]) {
    let collection = ctx.collection("resolved-vector");
    let schema =
        FieldSpec::f32_vector(3, true).with_index(FieldIndex::vector(VectorDistanceMetric::Cosine));
    ctx.client
        .collections()
        .create(&collection, [("embedding".into(), schema)], None)
        .await
        .unwrap();
    let from = ctx.cli.parquet(
        "vectors.parquet",
        "SELECT 'one' AS id, [1.0, 0.0, 0.0]::FLOAT[] AS embedding",
    );
    let preview = ctx
        .cli
        .command()
        .args([
            "-o",
            "json",
            "import",
            &from,
            "--to",
            &collection,
            "--dry-run",
        ])
        .args(field)
        .success();
    let plan = preview.plan();
    let resolved = &plan.target.fields["embedding"].schema;
    let destination = ctx.client.collections().get(&collection).await.unwrap();
    assert!(resolved.required);
    assert!(
        topk::import::plan::Field::new(resolved.clone()).matches(&destination.schema["embedding"])
    );
}

#[rstest_ctx(Scratch)]
fn import_help_shows_examples(ctx: &Scratch) {
    let help = ctx.command().args(["import", "--help"]).success().stdout;
    assert!(help.contains("Examples:"), "{help}");
    assert!(help.contains("postgres://user:pw@host/db#orders"), "{help}");
    assert!(help.contains("es://localhost:9200/articles"), "{help}");
    assert!(help.contains("-f embedding=vector:cosine"), "{help}");
}

#[test]
fn import_argument_schema() {
    ImportArgs::augment_args(Command::new("import")).debug_assert();
}
