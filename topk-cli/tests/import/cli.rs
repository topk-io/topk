use crate::import::common::seed::Backend;
use crate::import::common::seed::Backend::{Mongo, Postgres};
use crate::import::common::seed::{mongo, pg};
use crate::import::common::*;
use base64::Engine;
use serde_json::{json, Value as JsonValue};
use test_macros::rstest_ctx;
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
        .failure(1);
    let summary = done.summary();
    let outcome = &summary[&collection];
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
        .args(["-o", "json", "import", &scheme, &source.from, "--dry-run"])
        .success();
    let plan = run.plan();
    let found: Vec<_> = plan
        .collections
        .values()
        .map(|target| target.from.as_str())
        .collect();
    assert_eq!(found, [source.from.as_str()]);
}

#[rstest_ctx(Ctx)]
async fn previews_carry_no_credentials(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let source = ctx.seed(Postgres, "books", books()).await;
    let printed = ctx
        .command()
        .args(["import", &source.url, &source.from, "--dry-run"])
        .success()
        .stdout;
    assert!(
        !printed.contains("postgres"),
        "a preview names what to import, never where to connect:\n{printed}"
    );
}

#[rstest_ctx(Scratch)]
fn a_cli_filter_needs_a_single_object(ctx: &Scratch) {
    let base = ctx.name("filt");
    let columns = "(id INTEGER PRIMARY KEY, title TEXT)";
    let _tables = pg::Pg::temp(&[
        (format!("public.{base}_a"), columns),
        (format!("public.{base}_b"), columns),
    ]);

    let run = ctx
        .command()
        .args([
            "import",
            &pg::Pg::URL,
            &format!("{base}*"),
            "--filter",
            "id > 1",
            "--dry-run",
        ])
        .failure(1);

    assert!(run.stderr.contains("applies to a single object"), "{run:?}");
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
    let target = &preview.plan().collections[&collection];
    assert_eq!(
        target.fields.keys().map(String::as_str).collect::<Vec<_>>(),
        ["title", "embedding"]
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
    assert_eq!(done.summary()[&collection].rows, 1);
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
#[case::keep("'hello'", "value", FieldSpec::text(false), json!("hello"))]
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
    let target = &run.plan().collections[&collection];
    assert_eq!(target.fields.len(), 1);
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
    assert_eq!(done.summary()[&collection].rows, 1);
    let schema = ctx
        .client
        .collections()
        .get(&collection)
        .await
        .unwrap()
        .schema;
    assert_eq!(schema["value"].data_type, expected.data_type);
    assert_eq!(schema["value"].index, expected.index);
    assert_eq!(
        ctx.docs(&collection, ["one"]).await[0]["value"],
        expected_value
    );
}

#[rstest_ctx(Ctx)]
#[case::keyword("'hello'", "keyword", FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Text)), json!("hello"))]
#[case::typed_semantic("'hello'", "text:semantic", FieldSpec::text(false).with_index(FieldIndex::semantic()), json!("hello"))]
#[case::semantic("'hello'", "semantic", FieldSpec::text(false).with_index(FieldIndex::semantic()), json!("hello"))]
#[case::ngram("'hello'", "ngram", FieldSpec::text(false).with_index(FieldIndex::ngram()), json!("hello"))]
#[case::vector_cosine("[1.0,2.0]", "vector:cosine", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::Cosine)), json!([1.0, 2.0]))]
#[case::vector_euclidean("[1.0,2.0]", "vector:2:euclidean", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean)), json!([1.0, 2.0]))]
#[case::vector_dot_product("[1.0,2.0]", "vector:2:dot_product", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)), json!([1.0, 2.0]))]
#[case::vector_hamming("[1.0,2.0]", "vector:2:hamming", FieldSpec::f32_vector(2, false).with_index(FieldIndex::vector(VectorDistanceMetric::Hamming)), json!([1.0, 2.0]))]
#[case::matrix_maxsim("'[1,2,3,4]'", "matrix:2:maxsim", FieldSpec::matrix(false, 2, MatrixValueType::F32).with_index(FieldIndex::multi_vector(MultiVectorDistanceMetric::Maxsim, None, None, None)), json!([[1.0, 2.0], [3.0, 4.0]]))]
#[case::sparse_dot_product("'{\"1\":1.5}'", "sparse:dot_product", FieldSpec::f32_sparse_vector(false).with_index(FieldIndex::vector(VectorDistanceMetric::DotProduct)), json!({"1": 1.5}))]
async fn field_indexes_import(
    ctx: &Ctx,
    #[case] expression: &str,
    #[case] choice: &str,
    #[case] expected: FieldSpec,
    #[case] expected_value: JsonValue,
) {
    let from = ctx.cli.parquet(
        "index.parquet",
        &format!("SELECT 'one' AS id, {expression} AS value"),
    );
    let collection = ctx.collection("index");
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
            &format!("value={choice}"),
            "--yes",
        ])
        .success();
    assert_eq!(done.summary()[&collection].rows, 1);
    let actual = ctx
        .client
        .collections()
        .get(&collection)
        .await
        .unwrap()
        .schema;
    assert!(
        topk::import::plan::Field::new(expected).matches(&actual["value"]),
        "{choice}: {:?}",
        actual["value"]
    );
    assert_eq!(
        ctx.docs(&collection, ["one"]).await[0]["value"],
        expected_value
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
    assert_eq!(done.summary()[&dest].rows, 1);
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
    assert_eq!(done.summary()[&dest].rows, 1);
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
            "--dry-run",
        ])
        .success();
    assert_eq!(
        preview
            .plan()
            .collections
            .values()
            .next()
            .unwrap()
            .fields
            .len(),
        1
    );
    assert!(preview.stderr.contains("will be shortened"), "{preview:?}");
}

#[rstest_ctx(Ctx)]
async fn dry_run_requires_destination(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let with_key = ctx
        .command()
        .envs([("TOPK_API_KEY", "unused")])
        .args(["-o", "json", "import", &from, "--dry-run"])
        .success();
    let without_credentials = ctx
        .command()
        .env_remove("TOPK_PROJECT_ID")
        .envs([("TOPK_CONFIG_DIR", ctx.scratch())])
        .args(["import", &from, "--dry-run"])
        .failure(1);
    with_key.plan();
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
async fn dry_run_ignores_write_options(ctx: &Ctx, #[case] args: &[&str]) {
    let ctx = &ctx.cli;
    let from = ctx.seed_parquet("books", books());
    let state = ctx.scratch().join("runs");
    let run = ctx
        .command()
        .envs([("TOPK_IMPORT_STATE_DIR", &state)])
        .args(["import", &from, "--dry-run"])
        .args(args)
        .success();
    assert_eq!(run.preview().len(), 3);
    assert!(!state.exists());
}

#[rstest_ctx(Scratch)]
#[case::concurrency_cap(&["missing.parquet", "-c", "1025"], "1025 is not in 1..=1024", 2)]
#[case::preview_flag(&["x.parquet", "--preview"], "unexpected argument '--preview'", 2)]
#[case::resume_partition(&["--resume", "1234abcd", "--partition", "tenant"], "cannot be used with", 2)]
#[case::resume_fields(&["--resume", "1234abcd", "-f", "title"], "cannot be used with", 2)]
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
#[case::stdin_source(&["-", "--to", "c", "--id", "id", "--dry-run"], "cannot tell the file type", 2)]
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
        "source = \"\"\nregion = \"dev\"\n[plan.c]\nfrom = \"a.parquet\"\nid = \"_id\"\n[plan.c.fields]\n",
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
        .args(["import", &from, "--partition", "author", "--dry-run"])
        .success();
    assert!(preview.stderr.contains("# partition Lee"), "{preview:?}");
}

#[rstest_ctx(Scratch)]
async fn missing_id_rejected(ctx: &Scratch) {
    let from = ctx.file("no-id.csv", "a,b\nx,y");
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
        run.plan().collections["c"].filter.as_deref(),
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
        .args(["-o", "json", "import", &from, "--yes"])
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
    let files: Vec<_> = std::fs::read_dir(&state)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].extension().unwrap(), "lock");
    assert!(!ctx.exists(&collection).await);
}

#[rstest_ctx(Ctx)]
async fn unknown_field_column_fails_before_creating(ctx: &Ctx) {
    let from = ctx.cli.file("books.csv", "id,title\n1,dune");
    let collection = ctx.collection("bad-field");
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
            "--rename",
            "nonexistent_col=ghost",
            "-f",
            "title,ghost",
            "--yes",
        ])
        .failure(1);
    assert!(
        done.stderr
            .contains("source field \"nonexistent_col\" was not found"),
        "{done:?}"
    );
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
            "-f",
            "title",
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
    assert_eq!(first.summary()[&collection].rows, 3);
    assert_eq!(narrowed.summary()[&collection].rows, 3);
    assert_eq!(full.summary()[&collection].rows, 3);
    let expected: Vec<_> = books().iter().map(|doc| json!(json(doc))).collect();
    assert_eq!(ctx.docs(&collection, BOOK_IDS).await, expected);
}

#[rstest_ctx(Ctx)]
#[case::text("text")]
#[case::json("json")]
async fn skipped_rows_report_failure(ctx: &Ctx, #[case] format: &str) {
    let from = ctx.cli.file("bad-row.csv", "id,n\n1,5\n2,no\n3,7");
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
        .failure(1);
    assert!(output.stderr.contains("skipped"), "{output:?}");
    match format {
        "text" => {
            assert!(output.stdout.is_empty(), "{output:?}");
            assert!(
                output.stderr.contains("2 rows written, 1 failed"),
                "{output:?}"
            );
        }
        "json" => {
            let summary = output.summary();
            let outcome = &summary[&collection];
            assert_eq!((outcome.rows, outcome.failed), (2, 1));
            assert!(!output.stderr.contains("rows written"), "{output:?}");
        }
        _ => unreachable!(),
    }
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
    let output = ctx.command().args(["import", &from, "--dry-run"]).success();
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
        output
            .stderr
            .matches(&format!("{collection}: 3 rows written, 0 failed"))
            .count(),
        1,
        "{output:?}"
    );
}

#[rstest_ctx(Ctx)]
async fn failure_prints_resume_command(ctx: &Ctx) {
    let from = ctx.cli.file("bad-row.csv", "id,n\n1,not-a-number");
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
        output
            .stderr
            .contains(&format!("topk import --resume {}", output.id())),
        "{output:?}"
    );
    assert!(ctx.cli.state_path(output.id()).exists());
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
    assert_eq!(preview.plan().collections[&collection].id, "_id");
    if let Some(limit) = limit {
        assert_eq!(
            preview.plan().collections[&collection].limit,
            Some(limit.parse().unwrap())
        );
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
    assert_eq!(done.summary()[&collection].rows, 1);
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
async fn unsupported_lists_are_skipped_in_plan(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let from = ctx.file(
        "lists.jsonl",
        r#"{"id":"one","title":"book","flags":[true],"messages":[{"role":"user"}]}"#,
    );
    let run = ctx
        .command()
        .env_remove("RUST_LOG")
        .args(["import", &from, "--id", "id", "--dry-run"])
        .success();
    assert!(!run.stdout.contains("flags"), "{run:?}");
    assert!(!run.stdout.contains("messages"), "{run:?}");
    assert!(run.stderr.contains("skipping column \"flags\""), "{run:?}");
    assert!(
        run.stderr.contains("skipping column \"messages\""),
        "{run:?}"
    );
    assert!(run.stderr.contains("book"), "{run:?}");
}

#[rstest_ctx(Scratch)]
async fn empty_database_names_source(ctx: &Scratch) {
    let path = ctx.file("empty.db", "");
    let url = format!("sqlite:{path}");
    let run = ctx.command().args(["import", &url, "--dry-run"]).failure(1);
    assert!(run.stderr.contains(&path), "{run:?}");
    assert!(run.stderr.contains("no objects"), "{run:?}");
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
        &state.progress[&collection],
        topk::import::state::Progress::After(checkpoint) if checkpoint.consumed == 11
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
            "--continue-on-error",
            "-o",
            "json",
        ])
        .failure(1);
    let summary = resumed.summary();
    assert_eq!(
        (summary[&collection].rows, summary[&collection].failed),
        ((consumed - 12) as usize, 1)
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
async fn missing_file_is_readable(ctx: &Ctx) {
    let ctx = &ctx.cli;
    let path = ctx.scratch().join("missing.parquet");
    let path = path.to_str().unwrap();
    let output = ctx.command().args(["import", path, "--dry-run"]).failure(1);
    assert!(
        output
            .stderr
            .contains(&format!("error: file not found: {path}")),
        "{}",
        output.stderr
    );
    assert!(!output.stderr.contains("SELECT"), "{}", output.stderr);
}

#[rstest_ctx(Ctx)]
async fn dry_run_uses_destination(ctx: &Ctx) {
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
        .success();
    let plan = preview.plan();
    let resolved = &plan.collections[&collection].fields["embedding"].schema;
    let destination = ctx.client.collections().get(&collection).await.unwrap();
    assert_eq!(resolved, &destination.schema["embedding"]);
}
