use crate::import::common::seed::Backend;
use crate::import::common::seed::Backend::{Mongo, Postgres};
use crate::import::common::seed::{mongo, pg};
use crate::import::common::*;
use serde_json::json;
use test_macros::rstest_ctx;

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
#[rstest_ctx(Scratch)]
#[case::postgres_env(Postgres, "postgres://", &[
    ("PGHOST", "localhost"), ("PGPORT", "5433"), ("PGUSER", "postgres"),
    ("PGPASSWORD", "postgres"), ("PGDATABASE", "demo"),
])]
#[case::mongo_env(Mongo, "mongodb://", &[("MONGODB_URI", mongo::URL)])]
async fn bare_scheme_connects_from_environment(
    ctx: &Scratch,
    #[case] backend: Backend,
    #[case] scheme: &str,
    #[case] env: &[(&str, &str)],
) {
    let source = ctx.seed(backend, "books", books()).await;
    let run = ctx
        .command()
        .envs(env)
        .args(["import", &scheme, &source.from, "--dry-run"])
        .success();
    let spec = run.spec();
    let found: Vec<_> = spec
        .collections
        .values()
        .map(|target| target.from.as_str())
        .collect();
    assert_eq!(found, [source.from.as_str()]);
}

#[rstest_ctx(Scratch)]
async fn printed_specs_carry_no_credentials(ctx: &Scratch) {
    let source = ctx.seed(Postgres, "books", books()).await;
    let printed = ctx
        .command()
        .args(["import", &source.url, &source.from, "--dry-run"])
        .success()
        .stdout;
    assert!(
        !printed.contains("postgres"),
        "a spec names what to import, never where to connect:\n{printed}"
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

#[rstest_ctx(Scratch)]
async fn deterministic_output(ctx: &Scratch) {
    let from = ctx.seed_parquet("books", books());
    let first = ctx
        .command()
        .args(["import", &from, "--to", "c", "--dry-run"])
        .success();
    let second = ctx
        .command()
        .args(["import", &from, "--to", "c", "--dry-run"])
        .success();
    let saved = ctx.file("import.toml", &first.stdout);
    let reopened = ctx
        .command()
        .args(["import", "-f", &saved, "--dry-run"])
        .success();
    assert_eq!(first.stdout, second.stdout);
    assert!(!first.preview().is_empty(), "documents preview on stderr");
    assert_eq!(
        reopened.stdout,
        ctx.read("import.toml"),
        "dry-run output must be a reusable spec"
    );
    assert_eq!(reopened.preview(), first.preview());
}

#[rstest_ctx(Scratch)]
async fn local_dry_run_ignores_existing_api_key(ctx: &Scratch) {
    let from = ctx.seed_parquet("books", books());
    let with_key = ctx
        .command()
        .envs([("TOPK_API_KEY", "unused")])
        .args(["import", &from, "--dry-run"])
        .success();
    let without_credentials = ctx
        .command()
        .env_remove("TOPK_PROJECT_ID")
        .envs([("TOPK_CONFIG_DIR", ctx.scratch())])
        .args(["import", &from, "--dry-run"])
        .success();
    with_key.spec();
    assert_eq!(without_credentials.stdout, with_key.stdout);
}

#[rstest_ctx(Scratch)]
#[case::yes(&["--yes"])]
#[case::continue_on_error(&["--continue-on-error"])]
#[case::concurrency(&["--concurrency", "2"])]
#[case::batch_bytes(&["--batch-bytes", "1MiB"])]
async fn dry_run_ignores_write_options(ctx: &Scratch, #[case] args: &[&str]) {
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
#[case::resume_edited_spec(&["--resume", "1234abcd", "-f", "edited.toml"], "cannot be used with", 2)]
#[case::resume_dry_run(&["--resume", "1234abcd", "--dry-run"], "cannot be used with", 2)]
#[case::resume_path(&["--resume", "../escape"], "run id must be eight lowercase hex digits", 2)]
#[case::missing_spec(&["-f", "/nope/spec.toml"], "/nope/spec.toml", 1)]
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
    ctx.file("state/1234abcd.toml",
        "source = ''\nregion = 'dev'\n[spec.c]\nfrom = 'a.parquet'\nid = '_id'\n[spec.c.fields]\ntitle = { type = 'text' }\n");
    let run = ctx
        .command()
        .args(["-o", "json", "import", "--resume", "1234abcd"])
        .failure(1);
    assert!(
        run.stderr.contains("invalid or older saved state"),
        "{run:?}"
    );
    assert!(run.stderr.contains("project_id"), "{run:?}");
}

#[rstest_ctx(Scratch)]
async fn bare_spec_rejects_non_file_source(ctx: &Scratch) {
    let spec = ctx.file(
        "import.toml",
        r#"
[c]
from = "topk://dev/books"
id = "_id"
[c.fields]
title = { type = "text" }
"#,
    );

    let run = ctx
        .command()
        .args(["import", "-f", &spec, "--dry-run"])
        .failure(1);
    assert!(run.stderr.contains("topk://dev/books"), "{run:?}");
    assert!(run.stderr.contains("pass it as the source"), "{run:?}");
    assert!(run.stderr.contains("--spec <spec>"), "{run:?}");
}

#[rstest_ctx(Scratch)]
async fn file_source_does_not_restrict_spec_targets(ctx: &Scratch) {
    let first = ctx.seed_parquet("first", books());
    let second = ctx.seed_parquet("second", rows(1, 3));
    let spec = ctx.file(
        "import.toml",
        &format!(
            r#"[c]
from = {second:?}
id = "_id"

[c.fields]
name = {{ type = 'text' }}
"#
        ),
    );
    let run = ctx
        .command()
        .args(["import", &first, "-f", &spec, "--dry-run"])
        .success();
    assert_eq!(
        run.preview()
            .iter()
            .map(|doc| doc["_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
}

#[rstest_ctx(Scratch)]
async fn partition_column_previews(ctx: &Scratch) {
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

/// `--filter` is a CLI flag copied onto every discovered target, so the printed
/// spec carries it and the preview reads through it.
#[rstest_ctx(Scratch)]
async fn a_cli_filter_reaches_the_target(ctx: &Scratch) {
    let from = ctx.seed_parquet("books", books());
    let run = ctx
        .command()
        .args([
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
        run.spec().collections["c"].filter.as_deref(),
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
    let spec = ctx.cli.file(
        "import.toml",
        &format!(
            r#"[{collection}]
from = {from:?}
id = "id"

[{collection}.fields]
title = {{ type = 'text' }}
ghost = {{ type = 'text', from = 'nonexistent_col' }}
"#
        ),
    );
    // Without validation this "succeeds", writing a silent null for `ghost`.
    let done = ctx
        .cli
        .command()
        .args(["-o", "json", "import", &from, "-f", &spec, "--yes"])
        .failure(1);
    assert!(
        done.stderr.contains(r#"reads column "nonexistent_col""#)
            && done.stderr.contains("available: id, title"),
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
    let spec = ctx.cli.file(
        "import.toml",
        &format!(
            r#"[{collection}]
from = {from:?}
id = "_id"

[{collection}.fields]
title = {{ type = 'text' }}
"#
        ),
    );
    let narrowed = ctx
        .cli
        .command()
        .args(["-o", "json", "import", "-f", &spec, "--yes"])
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
    let spec = ctx.cli.file(
        "import.toml",
        &format!(
        "[{collection}]\nfrom = {from:?}\nid = 'id'\n[{collection}.fields]\nn = {{ type = 'int' }}"
    ),
    );
    let output = ctx
        .cli
        .command()
        .args([
            "import",
            "-f",
            &spec,
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

#[rstest_ctx(Scratch)]
async fn preview_elides_large_values(ctx: &Scratch) {
    let from = ctx.parquet(
        "data.parquet",
        "SELECT 1 AS id, repeat('é', 300) AS title, range(10) AS numbers",
    );
    let output = ctx.command().args(["import", &from, "--dry-run"]).success();
    output.spec();
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
    let spec = ctx.cli.file(
        "import.toml",
        &format!(
            r#"[{collection}]
from = {from:?}
id = "id"

[{collection}.fields]
n = {{ type = 'int' }}
"#
        ),
    );
    let output = ctx
        .cli
        .command()
        .args(["-o", "json", "import", "-f", &spec, "--yes"])
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
        .args(["import", &from, "--to", &collection, "--dry-run"])
        .args(limit.into_iter().flat_map(|limit| ["--limit", limit]))
        .success();
    assert_eq!(preview.spec().collections[&collection].id, "_id");
    if let Some(limit) = limit {
        assert_eq!(
            preview.spec().collections[&collection].limit,
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

#[rstest_ctx(Scratch)]
async fn unsupported_lists_are_skipped_in_plan(ctx: &Scratch) {
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

#[rstest_ctx(Scratch)]
#[case::positional_file(false)]
#[case::spec_file(true)]
async fn missing_file_is_readable(ctx: &Scratch, #[case] spec: bool) {
    let path = ctx.scratch().join("missing.parquet");
    let path = path.to_str().unwrap();
    let command = ctx.command().args(["import", "--dry-run"]);
    let output = if spec {
        let spec = ctx.file(
            "missing.toml",
            &format!("[docs]\nfrom = {path:?}\nid = 'id'\n[docs.fields.text]\ntype = 'text'\n"),
        );
        command.args(["--spec", &spec]).failure(1)
    } else {
        command.args([path]).failure(1)
    };
    assert!(
        output
            .stderr
            .contains(&format!("error: file not found: {path}")),
        "{}",
        output.stderr
    );
    assert!(!output.stderr.contains("SELECT"), "{}", output.stderr);
}
