use crate::common::seed::pg;
use crate::common::*;
use serde_json::json;
use test_context::test_context;
use topk::import::{Spec, Target};

#[test_context(Scratch)]
#[tokio::test]
async fn limit(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("big", rows(1, 100)).await;
    let target = Target {
        limit: Some(5),
        ..object
    };
    assert_eq!(stream_docs(&target).await.unwrap().len(), 5);
}

#[test_context(Scratch)]
#[tokio::test]
async fn glob(ctx: &mut Scratch) {
    for (i, base) in [1_u64, 11, 21].iter().enumerate() {
        ctx.seed_parquet(&format!("part_{i}"), rows(*base, 3)).await;
    }
    let target = Target {
        from: format!("{}/part_*.parquet", ctx.scratch().display()),
        ..Default::default()
    };
    assert_eq!(
        stream_docs(&target).await.unwrap().len(),
        9,
        "all 3 files × 3 rows"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn deterministic_output(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let spec = ctx.target_spec("c", object);

    let runs: Vec<String> = (0..3)
        .map(|_| ok(&["import", "-f", &spec, "--dry-run"], &[]))
        .collect();
    assert_eq!(runs[0], runs[1], "printed spec differs between runs");
    assert_eq!(runs[1], runs[2], "printed spec differs between runs");
    assert_eq!(
        toml::from_str::<Spec>(&runs[0])
            .expect("the printed spec re-parses")
            .collections
            .len(),
        1
    );

    let docs = previewed(None, &spec, &[]);
    assert!(!docs.is_empty(), "documents preview on stderr");
}

#[test_context(Scratch)]
#[tokio::test]
async fn preview_is_capped(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("big", rows(1, 20)).await;
    let spec = ctx.target_spec("c", object);
    assert_eq!(previewed(None, &spec, &[]).len(), 5);
}

#[test_context(Scratch)]
#[tokio::test]
async fn decimal_list(ctx: &mut Scratch) {
    let target = target(
        &ctx.sql_parquet(
            "decimals",
            "SELECT 1 AS id, [1.50, 2.25, 3.00]::DECIMAL(10,2)[] AS prices",
        ),
        "id",
        r#"prices = { type = "float_list" }"#,
    );
    let docs = stream_docs(&target).await.unwrap();
    assert_eq!(docs["1"], json!({"_id": "1", "prices": [1.5, 2.25, 3.0]}));
}

#[test_context(Scratch)]
#[tokio::test]
async fn bytes_as_text(ctx: &mut Scratch) {
    let target = target(
        &ctx.sql_parquet(
            "bytes",
            "SELECT 1 AS id, 'hello'::BLOB AS a, '42'::BLOB AS b",
        ),
        "id",
        r#"a = { type = "text" }
           b = { type = "text" }"#,
    );
    let docs = stream_docs(&target).await.unwrap();
    assert_eq!(docs["1"], json!({"_id": "1", "a": "hello", "b": "42"}));
}

#[test_context(Scratch)]
#[tokio::test]
async fn missing_id(ctx: &mut Scratch) {
    // A ragged csv sniffs to a single column named after the whole header, so
    // the declared id is not there. The catalog catches it before any scan.
    let file = ctx.scratch().join("ragged.csv");
    std::fs::write(&file, "id,a,b\n1,x,y\n2,z\n").unwrap();
    let err = fails(
        &[
            "import",
            &file.display().to_string(),
            "--id",
            "id",
            "--dry-run",
        ],
        &[],
    );
    assert!(err.contains("available: id,a,b"), "got: {err}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn id_placeholder(ctx: &mut Scratch) {
    let file = ctx.scratch().join("rows.csv");
    std::fs::write(&file, "a,b\nx,y\n").unwrap();
    let spec = ctx.target_spec(
        "c",
        target(
            &file.display().to_string(),
            "<column>",
            r#"a = { type = "text" }"#,
        ),
    );
    // A dry run renders the template, placeholder included, so it can be
    // captured and filled in; only a real import insists on a resolved id.
    let out = ok(&["import", "-f", &spec, "--dry-run"], &[]);
    assert!(
        out.contains(r#"_id = { from = "<column>" }"#),
        "dry-run must render the placeholder spec:\n{out}"
    );
    let err = fails(&["import", "-f", &spec, "--yes"], &[]);
    assert!(
        err.contains("--id"),
        "a real import must point at --id:\n{err}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn bad_filter_names_the_filter(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let target = Target {
        filter: Some("bogus(".to_string()),
        ..object
    };

    let message = stream_docs(&target).await.unwrap_err().to_string();
    assert!(message.contains("bogus("), "got: {message}");
    assert!(!message.contains("SELECT"), "got: {message}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn missing_spec_names_the_path(_ctx: &mut Scratch) {
    let err = fails(&["import", "-f", "/nope/spec.toml"], &[]);
    assert!(err.contains("/nope/spec.toml"), "got: {err}");
}

/// `--filter` is a CLI flag copied onto every discovered target, so the printed
/// spec carries it and the preview reads through it.
#[test_context(Scratch)]
#[tokio::test]
async fn a_cli_filter_reaches_the_target(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let out = run(
        &[
            "import",
            &object.from,
            "--to",
            "c",
            "--filter",
            "published_year > 1950",
            "--dry-run",
            "--preview",
        ],
        &[],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "got:\n{stderr}");
    assert!(
        stdout.contains(r#"filter = "published_year > 1950""#),
        "the printed spec must carry the flag:\n{stdout}"
    );
    let previewed: Vec<String> = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| l.trim_start().starts_with('{'))
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).expect("dry-run json")["_id"]
                .as_str()
                .expect("string id")
                .to_string()
        })
        .collect();
    assert_eq!(previewed, ["mockingbird"]);
}

#[tokio::test]
async fn a_cli_filter_needs_a_single_object() {
    let base = unique_name("filt");
    let columns = "(id INTEGER PRIMARY KEY, title TEXT)";
    let _tables = pg::Pg::temp(&[
        (format!("public.{base}_a"), columns),
        (format!("public.{base}_b"), columns),
    ]);

    let err = fails(
        &[
            "import",
            pg::Pg::URL,
            &format!("{base}*"),
            "--filter",
            "id > 1",
            "--dry-run",
        ],
        &[],
    );
    assert!(err.contains("applies to a single object"), "got: {err}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn timestamp_previews_as_a_date(ctx: &mut Scratch) {
    let target = target(
        &ctx.sql_parquet("stamps", "SELECT 1 AS id, 1369307590000 AS at"),
        "id",
        r#"at = { type = "timestamp" }"#,
    );
    let spec = ctx.target_spec("c", target);
    let docs = previewed(None, &spec, &[]);
    assert_eq!(docs["1"], json!({"_id": "1", "at": "2013-05-23T11:13:10Z"}));
}

/// Every field of a wide record prints, each cut to its share of the line.
#[test_context(Scratch)]
#[tokio::test]
async fn a_wide_record_elides_rather_than_drops(ctx: &mut Scratch) {
    let target = target(
        &ctx.sql_parquet(
            "wide",
            "SELECT 1 AS id, 1369307590000 AS at, repeat('x', 500) AS body, \
             [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0] AS vec, \
             'first' AS a, 'second' AS b",
        ),
        "id",
        r#"at = { type = "timestamp" }
           body = { type = "text" }
           vec = { type = "float_list" }
           a = { type = "text" }
           b = { type = "text" }"#,
    );
    let spec = ctx.target_spec("c", target);
    let out = run(&["import", "-f", &spec, "--dry-run", "--preview"], &[]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let record = stderr
        .lines()
        .find(|line| line.starts_with('{'))
        .expect("a document previewed");

    for field in ["_id", "at", "body", "vec", "a", "b"] {
        assert!(record.contains(&format!("{field:?}:")), "{field} dropped");
    }
    assert!(
        record.contains(r#""at": "2013-05-23T11:13:10Z""#),
        "the timestamp was cut: {record}"
    );
    assert!(
        record.contains("more]"),
        "the list kept every value: {record}"
    );
    assert!(!record.contains(&"x".repeat(40)), "the text was not cut");
    assert!(record.len() < 300, "{} chars: {record}", record.len());
}

/// The two flags are independent: `--dry-run` decides whether anything is
/// written, `--preview` whether documents print.
#[test_context(Scratch)]
#[tokio::test]
async fn dry_run_and_preview_compose(ctx: &mut Scratch) {
    let target = target(
        &ctx.sql_parquet("wide", "SELECT 1 AS id, repeat('x', 500) AS body"),
        "id",
        r#"body = { type = "text" }"#,
    );
    let spec = ctx.target_spec("c", target);
    let out = run(&["import", "-f", &spec, "--dry-run", "--preview"], &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stdout.contains("[c]"), "the spec is stdout:\n{stdout}");
    assert!(
        stderr.lines().any(|l| l.starts_with('{')),
        "the documents are stderr:\n{stderr}"
    );

    let out = run(
        &[
            "import",
            "-f",
            &spec,
            "--dry-run",
            "--preview",
            "-o",
            "json",
        ],
        &[],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let line = stderr
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("a document previewed");
    let doc: serde_json::Value = serde_json::from_str(line).expect("ndjson for jq");
    assert_eq!(
        doc["body"].as_str().map(str::len),
        Some(500),
        "still elided"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn a_bare_dry_run_reads_no_rows(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let spec = ctx.target_spec("c", object);
    let out = run(&["import", "-f", &spec, "--dry-run"], &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let printed = format!("{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("[c]"), "the spec prints:\n{stdout}");
    assert!(
        !printed.lines().any(|l| l.trim_start().starts_with('{')),
        "no row should be read:\n{printed}"
    );
}
