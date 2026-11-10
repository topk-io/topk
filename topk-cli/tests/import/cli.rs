use crate::import::common::gate::Gate;
use crate::import::common::seed::Backend;
use crate::import::common::seed::Backend::{Mongo, Postgres, Sqlite, S3};
use crate::import::common::seed::{mongo, pg, rustfs};
use crate::import::common::*;
use rstest::rstest;
use serde_json::json;
use std::collections::BTreeMap;
use std::os::unix::process::ExitStatusExt;
use test_context::test_context;
use test_macros::rstest_ctx;
use tokio::time::{timeout, Duration};

const BOOK_IDS: [&str; 3] = ["mockingbird", "nineteen_eighty_four", "pride"];

/// A bare scheme takes its connection from the source's usual environment.
async fn bare_scheme_connects_from_environment(
    backend: Backend,
    scheme: &str,
    env: &[(&str, &str)],
) {
    let source = backend.seed(books()).await;
    let run = topk!(topk().envs(env); "import", scheme, source.from, "--dry-run").unwrap();
    let found: Vec<_> = run
        .spec()
        .collections
        .into_values()
        .map(|target| target.from)
        .collect();
    assert_eq!(found, [source.from]);
}

#[tokio::test]
async fn postgres_connects_from_environment() {
    let env = [
        ("PGHOST", "localhost"),
        ("PGPORT", "5433"),
        ("PGUSER", "postgres"),
        ("PGPASSWORD", "postgres"),
        ("PGDATABASE", "demo"),
    ];

    bare_scheme_connects_from_environment(Postgres, "postgres://", &env).await;
}

#[tokio::test]
async fn printed_specs_carry_no_credentials() {
    let source = Postgres.seed(books()).await;
    let printed = topk!(source.import(); source.from, "--dry-run")
        .unwrap()
        .stdout;
    assert!(
        !printed.contains("postgres"),
        "a spec names what to import, never where to connect:\n{printed}"
    );
}

#[test]
fn a_cli_filter_needs_a_single_object() {
    let base = unique_name("filt");
    let columns = "(id INTEGER PRIMARY KEY, title TEXT)";
    let _tables = pg::Pg::temp(&[
        (format!("public.{base}_a"), columns),
        (format!("public.{base}_b"), columns),
    ]);

    let run = topk!(
        "import",
        pg::Pg::URL,
        format!("{base}*"),
        "--filter",
        "id > 1",
        "--dry-run"
    )
    .unwrap_err();

    assert!(run.stderr.contains("applies to a single object"), "{run:?}");
}

#[test_context(Ctx)]
#[tokio::test]
async fn http_roundtrip(ctx: &mut Ctx) {
    let object = S3.seed(books()).await;
    // Public-read policy so the same object is a plain anonymous http GET.
    std::process::Command::new("curl")
        .args([
            "-s", "-X", "PUT",
            &format!("{}/topk-it/?policy", rustfs::S3::ENDPOINT),
            "--aws-sigv4", "aws:amz:us-east-1:s3",
            "--user", "rustfsadmin:rustfsadmin",
            "-d",
            r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"AWS":["*"]},"Action":["s3:GetObject"],"Resource":["arn:aws:s3:::topk-it/*"]}]}"#,
        ])
        .output()
        .unwrap();
    let collection = ctx.collection("http");
    let spec = ctx.multi_spec([(
        collection.clone(),
        target(
            &object.from.replace("s3:/", rustfs::S3::ENDPOINT),
            "_id",
            "title = { type = 'text' }",
        ),
    )]);

    topk!("import", "-f", spec, "--yes").unwrap();

    let docs = ctx.docs(&collection, ["mockingbird"]).await;
    assert_eq!(docs[0]["title"], json!("To Kill a Mockingbird"));
}

#[test_context(Scratch)]
#[tokio::test]
async fn unqualified_database_table_still_imports(ctx: &mut Scratch) {
    let seeded = Sqlite.seed(books()).await;
    let unqualified = seeded.from.rsplit('.').next().unwrap();
    let spec = ctx.spec_file(&spec_toml(unqualified, "_id", "title = { type = 'text' }"));

    topk!(seeded.import(); "-f", spec, "--dry-run").unwrap();
}

#[tokio::test]
async fn mongo_connects_from_environment() {
    let env = [("MONGODB_URI", mongo::Mongo::URL)];

    bare_scheme_connects_from_environment(Mongo, "mongodb://", &env).await;
}

#[test_context(Scratch)]
#[tokio::test]
async fn deterministic_output(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("books", books()).await;
    let first = topk!("import", from, "--to", "c", "--dry-run").unwrap();
    let second = topk!("import", from, "--to", "c", "--dry-run").unwrap();
    let saved = ctx.spec_file(&first.stdout);
    let reopened = topk!("import", "-f", saved, "--dry-run").unwrap();
    assert_eq!(first.stdout, second.stdout);
    assert!(!first.docs().is_empty(), "documents preview on stderr");
    assert_eq!(
        reopened.stdout, first.stdout,
        "dry-run output must be a reusable spec"
    );
    assert_eq!(reopened.docs(), first.docs());
}

#[test_context(Scratch)]
#[tokio::test]
async fn local_dry_run_ignores_existing_api_key(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("books", books()).await;
    let with_key =
        topk!(topk().envs([("TOPK_API_KEY", "unused")]); "import", from, "--dry-run").unwrap();
    let without_credentials = topk!(topk().env_remove("TOPK_PROJECT_ID")
        .envs([("TOPK_CONFIG_DIR", ctx.scratch())]); "import", from, "--dry-run")
    .unwrap();
    with_key.spec();
    assert_eq!(without_credentials.stdout, with_key.stdout);
}

#[rstest_ctx(Scratch)]
#[case::yes(&["--yes"])]
#[case::continue_on_error(&["--continue-on-error"])]
#[case::concurrency(&["--concurrency", "2"])]
#[case::batch_bytes(&["--batch-bytes", "1MiB"])]
async fn dry_run_ignores_write_options(ctx: &mut Scratch, #[case] args: &[&str]) {
    let from = ctx.seed_parquet("books", books()).await;
    let state = ctx.scratch().join("runs");
    let run = CliOutput::of(
        topk()
            .envs([("TOPK_IMPORT_STATE_DIR", &state)])
            .args(["import", &from, "--dry-run"])
            .args(args),
    )
    .unwrap();
    assert_eq!(run.docs().len(), 3);
    assert!(!state.exists());
}

#[rstest]
#[case::preview_flag(&["x.parquet", "--preview"], "unexpected argument '--preview'")]
#[case::resume_partition(&["--resume", "1234abcd", "--partition", "tenant"], "cannot be used with")]
#[case::resume_edited_spec(&["--resume", "1234abcd", "-f", "edited.toml"], "cannot be used with")]
#[case::resume_dry_run(&["--resume", "1234abcd", "--dry-run"], "cannot be used with")]
#[case::resume_path(&["--resume", "../escape"], "run id must be eight lowercase hex digits")]
#[case::missing_spec(&["-f", "/nope/spec.toml"], "/nope/spec.toml")]
fn rejected_arguments(#[case] args: &[&str], #[case] fragment: &str) {
    let run = CliOutput::of(topk().args(["-o", "json"]).args(["import"]).args(args)).unwrap_err();
    assert!(run.stderr.contains(fragment), "{run:?}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn resume_rejects_state_without_project(ctx: &mut Scratch) {
    std::fs::write(ctx.state_path("1234abcd"),
        "source = ''\nregion = 'dev'\n[spec.c]\nfrom = 'a.parquet'\nid = '_id'\n[spec.c.fields]\ntitle = { type = 'text' }\n").unwrap();
    let run = topk!("import", "--resume", "1234abcd").unwrap_err();
    assert!(
        run.stderr.contains("invalid or older saved state"),
        "{run:?}"
    );
    assert!(run.stderr.contains("project_id"), "{run:?}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn bare_spec_rejects_non_file_source(ctx: &mut Scratch) {
    let spec = ctx.spec_file(&spec_toml(
        "topk://dev/books",
        "_id",
        "title = { type = 'text' }",
    ));
    let run = topk!("import", "-f", spec, "--dry-run").unwrap_err();
    assert!(run.stderr.contains("bare -f reads files only"), "{run:?}");
    assert!(run.stderr.contains("SOURCE -f spec.toml"), "{run:?}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn file_source_does_not_restrict_spec_targets(ctx: &mut Scratch) {
    let first = ctx.seed_parquet("first", books()).await;
    let second = ctx.seed_parquet("second", rows(1, 3)).await;
    let spec = ctx.spec("c", target(&second, "_id", "name = { type = 'text' }"));
    let run = topk!("import", first, "-f", spec, "--dry-run").unwrap();
    assert_eq!(
        run.docs()
            .iter()
            .map(|doc| doc["_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn partition_column_previews(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("books", books()).await;
    let preview = topk!("import", from, "--partition", "author", "--dry-run").unwrap();
    let missing = topk!("import", from, "--partition", "nope", "--dry-run").unwrap_err();
    assert!(preview.stderr.contains("# partition Lee"), "{preview:?}");
    assert!(
        missing.stderr.contains("partition column \"nope\""),
        "{missing:?}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn dry_run_is_capped(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("rows", rows(1, 20)).await;
    let preview = topk!("import", from, "--dry-run").unwrap();
    assert_eq!(preview.docs().len(), 5);
}

#[test_context(Scratch)]
#[tokio::test]
async fn id_placeholder(ctx: &mut Scratch) {
    let from = ctx.csv("no-id", "a,b\nx,y");
    let spec = ctx.spec("c", target(&from, "<column>", "a = { type = 'text' }"));
    // A dry run renders the template, placeholder included, so it can be
    // captured and filled in; only a real import insists on a resolved id.
    let preview = topk!("import", "-f", spec, "--dry-run").unwrap();
    let done = topk!("import", "-f", spec, "--yes", "--project-id", "p1").unwrap_err();
    assert_eq!(
        preview.spec().collections["c"].id.as_deref(),
        Some("<column>")
    );
    assert!(
        preview.stderr.contains("set `id` in the spec") && !preview.stderr.contains("--id"),
        "{preview:?}"
    );
    assert!(
        done.stderr.contains("set `id` in the spec") && !done.stderr.contains("--id"),
        "{done:?}"
    );
}

/// `--filter` is a CLI flag copied onto every discovered target, so the printed
/// spec carries it and the preview reads through it.
#[test_context(Scratch)]
#[tokio::test]
async fn a_cli_filter_reaches_the_target(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("books", books()).await;
    let run = topk!(
        "import",
        from,
        "--to",
        "c",
        "--filter",
        "published_year > 1950",
        "--dry-run"
    )
    .unwrap();
    assert_eq!(
        run.spec().collections["c"].filter.as_deref(),
        Some("published_year > 1950")
    );
    assert_eq!(
        run.docs()
            .iter()
            .map(|doc| doc["_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["mockingbird"]
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn empty_region_is_rejected(ctx: &mut Scratch) {
    let from = ctx.seed_parquet("books", books()).await;
    let run = topk!(topk().envs([("TOPK_REGION", "")]); "import", from, "--yes").unwrap_err();
    assert!(
        run.stderr
            .contains("a value is required for '--region <REGION>'"),
        "{run:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn yes_required_without_tty(ctx: &mut Ctx) {
    let from = ctx.seed_parquet("books", books()).await;
    let collection = ctx.collection("confirm");
    let state = ctx.scratch().join("unconfirmed");
    let run =
        topk!(topk().envs([("TOPK_IMPORT_STATE_DIR", &state)]); "import", from, "--to", collection)
            .unwrap_err();
    assert!(run.stderr.contains("pass --yes"), "{run:?}");
    assert_eq!(
        std::fs::read_dir(&state)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>(),
        [state.join(format!("{}.lock", run.id()))]
    );
    assert!(!ctx.exists(&collection).await);
}

#[rstest]
#[case::zero_limit(&["--limit", "0"], "invalid value '0'")]
#[case::concurrency_cap(&["missing.parquet", "-c", "1025"], "1025 is not in 1..=1024")]
fn invalid_flags(#[case] args: &[&str], #[case] fragment: &str) {
    let run = CliOutput::of(topk().args(["-o", "json"]).args(["import"]).args(args)).unwrap_err();
    assert!(run.stderr.contains(fragment), "{run:?}");
}

#[test_context(Ctx)]
#[tokio::test]
async fn blank_id(ctx: &mut Ctx) {
    let from = ctx.csv("blank-id", "id,name\n1,ok\n,blank-id");
    let collection = ctx.collection("blank-id");
    let spec = ctx.spec(&collection, target(&from, "id", "name = { type = 'text' }"));
    let done = topk!("import", "-f", spec, "--yes").unwrap_err();
    assert!(done.stderr.contains(r#"field "id""#), "{done:?}");
}

#[rstest_ctx(Ctx)]
async fn nan_rejected(ctx: &mut Ctx, #[values(true, false)] dry_run: bool) {
    let from = ctx.parquet("SELECT 1 AS id, 'nan'::DOUBLE AS x");
    let collection = ctx.collection("nan");
    let spec = ctx.spec(&collection, target(&from, "id", "x = { type = 'float' }"));
    let done = topk!(
        "import",
        "-f",
        spec,
        if dry_run { "--dry-run" } else { "--yes" }
    )
    .unwrap_err();
    assert!(done.stderr.contains("non-finite"), "{done:?}");
}

#[test_context(Ctx)]
#[tokio::test]
async fn unknown_field_column_fails_before_creating(ctx: &mut Ctx) {
    let from = ctx.csv("books", "id,title\n1,dune");
    let collection = ctx.collection("bad-field");
    let spec = ctx.spec(
        &collection,
        target(
            &from,
            "id",
            "title = { type = 'text' }\nghost = { type = 'text', from = 'nonexistent_col' }",
        ),
    );
    // Without validation this "succeeds", writing a silent null for `ghost`.
    let done = topk!("import", "-f", spec, "--yes").unwrap_err();
    assert!(
        done.stderr.contains(r#"reads column "nonexistent_col""#)
            && done.stderr.contains("available: id, title"),
        "{done:?}"
    );
    assert!(!ctx.exists(&collection).await);
}

#[test_context(Ctx)]
#[tokio::test]
async fn unknown_id_column_fails_before_creating(ctx: &mut Ctx) {
    let from = ctx.csv("books", "id,title\n1,dune");
    let collection = ctx.collection("bad-id");
    let run = topk!("import", from, "--to", collection, "--id", "nope", "--yes").unwrap_err();
    assert!(
        run.stderr.contains(r#"id column "nope""#) && run.stderr.contains("available: id, title"),
        "{run:?}"
    );
    assert!(!ctx.exists(&collection).await);
}

/// An upsert replaces the document, so a spec that drops a field clears it on
/// every row it rewrites — and the drift refusal advises dropping fields. Say so
/// before the prompt.
#[test_context(Ctx)]
#[tokio::test]
async fn reimport_replaces_fields_and_warns(ctx: &mut Ctx) {
    let from = ctx.seed_parquet("books", books()).await;
    let collection = ctx.collection("reimport");
    let first = topk!("import", from, "--to", collection, "--yes").unwrap();
    let spec = ctx.spec(
        &collection,
        target(&from, "_id", "title = { type = 'text' }"),
    );
    let narrowed = topk!("import", "-f", spec, "--yes").unwrap();
    assert!(
        narrowed
            .stderr
            .contains("in the collection but not in this spec"),
        "{narrowed:?}"
    );
    let docs = ctx.docs(&collection, BOOK_IDS).await;
    assert_eq!(docs.len(), 3);
    assert!(docs.iter().all(|doc| doc.as_object().unwrap().len() == 2));
    // The fields stay in the collection's schema, so a spec that declares them
    // all has nothing to clear and says nothing.
    let full = topk!("import", from, "--to", collection, "--yes").unwrap();
    assert_eq!(
        (
            first.rows(&collection),
            narrowed.rows(&collection),
            full.rows(&collection)
        ),
        (3, 3, 3)
    );
    assert!(!full.stderr.contains("not in this spec"), "{full:?}");
    let expected: Vec<_> = books().iter().map(|doc| json!(json(doc))).collect();
    assert_eq!(ctx.docs(&collection, BOOK_IDS).await, expected);
}

#[rstest_ctx(Ctx)]
#[case::text("text")]
#[case::json("json")]
async fn skipped_rows_report_failure(ctx: &mut Ctx, #[case] format: &str) {
    let from = ctx.csv("bad-row", "id,n\n1,5\n2,no\n3,7");
    let collection = ctx.collection("summary");
    let spec = ctx.spec(&collection, target(&from, "id", "n = { type = 'int' }"));
    let output = CliOutput::of(topk().args([
        "import",
        "-f",
        &spec,
        "--yes",
        "--continue-on-error",
        "-o",
        format,
    ]))
    .unwrap_err();
    assert_eq!(output.status.code(), Some(1));
    if format == "json" {
        assert_eq!(
            (output.rows(&collection), output.failed(&collection)),
            (2, 1)
        );
        assert!(!output.stderr.contains("rows written"), "{output:?}");
    } else {
        assert!(output.stdout.is_empty(), "{output:?}");
        assert!(
            output.stderr.contains("2 rows written, 1 failed"),
            "{output:?}"
        );
    }
    assert_eq!(ctx.docs(&collection, [1, 2, 3]).await.len(), 2);
}

#[test_context(Scratch)]
#[tokio::test]
async fn preview_elides_large_values(ctx: &mut Scratch) {
    let from = ctx.parquet("SELECT 1 AS id, repeat('é', 121) AS title, range(10) AS numbers");
    let output = topk!("import", from, "--dry-run").unwrap();
    output.spec();
    assert!(
        output.stderr.contains(&format!("{:?}…", "é".repeat(120))),
        "{output:?}"
    );
    assert!(
        output.stderr.contains("[0, 1, … 10 values, 8, 9]"),
        "{output:?}"
    );
    assert!(!output.stdout.contains('…'), "{output:?}");
}

#[test_context(Ctx)]
#[tokio::test]
async fn successful_text_summary(ctx: &mut Ctx) {
    let from = ctx.seed_parquet("books", books()).await;
    let collection = ctx.collection("text-summary");
    let output =
        CliOutput::of(topk().args(["import", &from, "--to", &collection, "--yes"])).unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        output
            .stderr
            .contains(&format!("{collection}: 3 rows written")),
        "{output:?}"
    );
}

#[test_context(Ctx)]
#[tokio::test]
async fn failure_prints_resume_command(ctx: &mut Ctx) {
    let from = ctx.csv("bad-row", "id,n\n1,not-a-number");
    let collection = ctx.collection("resume-hint");
    let spec = ctx.spec(&collection, target(&from, "id", "n = { type = 'int' }"));
    let output = topk!("import", "-f", spec, "--yes").unwrap_err();
    assert_eq!(output.status.code(), Some(1));
    assert!(
        output
            .stderr
            .contains(&format!("topk import --resume {}", output.id())),
        "{output:?}"
    );
    assert!(ctx.state_path(output.id()).exists());
}

#[rstest_ctx(Ctx)]
#[case::interrupt(libc::SIGINT)]
#[case::process_death(libc::SIGKILL)]
async fn stopped_process_resumes_committed_writes(ctx: &mut Ctx, #[case] signal: i32) {
    let from = ctx.seed_parquet("rows", rows(0, 6)).await;
    let collection = ctx.collection("signal-resume");
    let mut gate = Gate::start(ctx).await;
    let child = tokio::process::Command::new(env!("CARGO_BIN_EXE_topk"))
        .args([
            "import",
            &from,
            "--to",
            &collection,
            "--yes",
            "--batch-bytes",
            "1B",
            "-c",
            "1",
            "--host",
            gate.config.host(),
            "--https",
            "false",
        ])
        .env_remove("TOPK_API_KEY")
        .env("TOPK_IMPORT_STATE_DIR", state_dir())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let pid = child.id().unwrap();
    let mut first = gate.next().await;
    first.forward.send(Ok(())).unwrap();
    assert_eq!(
        timeout(Duration::from_secs(10), &mut first.committed)
            .await
            .unwrap()
            .unwrap(),
        Ok(())
    );
    assert_eq!(ctx.docs(&collection, 0..6).await.len(), 1);
    assert!(std::process::Command::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .status()
        .unwrap()
        .success());
    let stopped = CliOutput::from(
        timeout(Duration::from_secs(10), child.wait_with_output())
            .await
            .unwrap()
            .unwrap(),
    );
    let _ = first.reply.send(Ok(()));
    if signal == libc::SIGINT {
        assert_eq!(stopped.status.code(), Some(130), "{stopped:?}");
        assert!(
            stopped
                .stderr
                .contains(&format!("topk import --resume {}", stopped.id())),
            "{stopped:?}"
        );
    } else {
        assert_eq!(stopped.status.signal(), Some(libc::SIGKILL));
    }
    let id = stopped.id();
    assert!(ctx.state_path(id).exists());
    let resumed = topk!(topk().envs([("TOPK_PROJECT_ID", "changed"), ("TOPK_REGION", "changed")]);
        "import", "--resume", id)
    .unwrap();
    assert_eq!(resumed.rows(&collection), 6);
    assert_eq!(ctx.docs(&collection, 0..6).await.len(), 6);
    assert!(!ctx.state_path(id).exists());
}

#[rstest_ctx(Ctx)]
#[case::sqlite(Backend::Sqlite)]
#[case::postgres(Backend::Postgres)]
#[case::mysql(Backend::Mysql)]
#[case::mongo(Backend::Mongo)]
#[case::elasticsearch(Backend::Elasticsearch)]
#[case::parquet(Backend::Parquet)]
#[case::s3(Backend::S3)]
async fn imports(ctx: &mut Ctx, #[case] backend: Backend) {
    let source = backend.seed(books()).await;
    let collection = ctx.collection("source");
    let preview =
        topk!(source.import(); source.from, "--to", collection, "--limit", 2, "--dry-run").unwrap();
    let spec = preview.spec();
    let target = &spec.collections[&collection];
    assert_eq!(target.id.as_deref(), Some("_id"));
    assert_eq!(target.limit, Some(2));
    let types: BTreeMap<_, _> = target
        .fields
        .iter()
        .map(|(name, field)| (name.as_str(), field.ty.to_string()))
        .collect();
    assert_eq!(
        types,
        BTreeMap::from([
            ("author", "text".into()),
            ("in_print", backend.bool_type().into()),
            ("published_year", "int".into()),
            ("rating", "float".into()),
            ("title", "text".into()),
        ])
    );
    assert_eq!(preview.docs().len(), 2);
    assert!(!ctx.exists(&collection).await);
    let done = topk!(source.import(); source.from, "--to", collection, "--filter", backend.filter(), "--yes").unwrap();
    assert_eq!(done.rows(&collection), 1);
    let mut expected = json!({"_id": "mockingbird", "title": "To Kill a Mockingbird", "author": "Lee",
        "published_year": 1960, "rating": 4.3, "in_print": true});
    if matches!(backend, Backend::Sqlite) {
        expected["in_print"] = json!(1);
    }
    assert_eq!(
        ctx.docs(
            &collection,
            ["mockingbird", "nineteen_eighty_four", "pride"]
        )
        .await,
        [expected]
    );
}

#[test]
fn stdin_preview() {
    let run = topk!(topk().stdin(Some("{\"id\":\"1\",\"title\":\"book\"}\n"));
        "import", "-", "--to", "c", "--id", "id", "--dry-run")
    .unwrap();
    assert_eq!(run.docs(), vec![json!({"_id": "1", "title": "book"})]);
}
