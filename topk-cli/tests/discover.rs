mod common;

use common::seed::pg;
use common::*;
use test_context::test_context;
use topk::import::{Error, Uri};

async fn discover_err(locator: &str, pattern: &str) -> String {
    let uri: Uri = locator.parse().expect("source uri parses");
    match topk::import::discover(&uri, &[pattern.to_string()], None, None).await {
        Err(Error::InvalidArgument(message)) => message,
        Err(other) => panic!("expected InvalidArgument, got {other:?}"),
        Ok(spec) => panic!(
            "expected discover to fail, got {} collection(s)",
            spec.collections.len()
        ),
    }
}

#[test_context(Scratch)]
#[tokio::test]
async fn float_lists_stay_lists(ctx: &mut Scratch) {
    let path = ctx.sql_parquet(
        "emb",
        "SELECT i AS id, [0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8]::FLOAT[] AS embedding \
         FROM range(3) t(i)",
    );

    let spec = discover_spec(&path, None).await;
    let field = &spec.collections["emb"].fields["embedding"];
    assert_eq!(field.ty.to_string(), "float_list");
    assert_eq!(field.dim, None);

    let printed = discover(&path, None);
    assert!(
        printed.contains("for vector search use"),
        "float lists must say how to become vectors:\n{printed}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn glob_needs_a_name(ctx: &mut Scratch) {
    for i in 0..2 {
        ctx.seed_parquet(&format!("part_{i}"), books()).await;
    }
    let pattern = format!("{}/part_*.parquet", ctx.scratch().display());

    let message = discover_err(&pattern, "*").await;
    assert!(message.contains("pass --to <name>"), "got: {message}");

    let uri: Uri = pattern.parse().expect("source uri parses");
    let spec = topk::import::discover(&uri, &[], Some("parts"), None)
        .await
        .expect("--to names the collection");
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), ["parts"]);
}

#[tokio::test]
async fn inline_rename() {
    let table = pg::Pg::seed_keyed_on("sku");
    let uri: Uri = pg::Pg::URL.parse().expect("source uri parses");
    let spec = topk::import::discover(&uri, &[format!("{table}=renamed")], None, None)
        .await
        .expect("discover");
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), ["renamed"]);
}

#[tokio::test]
async fn printed_specs_carry_no_credentials() {
    let table = pg::Pg::seed();
    let printed = discover(pg::Pg::URL, Some(&table));
    assert!(
        !printed.contains("postgres"),
        "a spec names what to import, never where to connect:\n{printed}"
    );
}

#[tokio::test]
async fn primary_key_id() {
    let table = pg::Pg::seed_keyed_on("sku");
    let spec = discover_spec(pg::Pg::URL, Some(&table)).await;
    let collection = table.rsplit('.').next().unwrap();
    assert_eq!(
        spec.collections.get(collection).unwrap().id.as_deref(),
        Some("sku")
    );
}

#[tokio::test]
async fn composite_key() {
    let table = pg::Pg::seed_composite();
    let spec = discover_spec(pg::Pg::URL, Some(&table)).await;
    let collection = table.rsplit('.').next().unwrap();
    assert_eq!(
        spec.collections.get(collection).unwrap().id.as_deref(),
        Some("<column>")
    );
}

#[tokio::test]
async fn glob_selection() {
    let a = pg::Pg::seed_keyed_on("sku");
    let b = pg::Pg::seed_composite();

    // both tables carry a random suffix; match each by its distinct prefix
    let keyed_prefix = format!("{}*", &a[..a.len() - 4]);
    let spec = discover_spec(pg::Pg::URL, Some(&keyed_prefix)).await;
    assert!(
        spec.collections.values().any(|t| t.from == a),
        "glob missed {a}"
    );
    assert!(
        spec.collections.values().all(|t| t.from != b),
        "glob matched too much"
    );
}

#[tokio::test]
async fn nothing_matched() {
    let _table = pg::Pg::seed();
    let message = discover_err(pg::Pg::URL, "public.definitely_not_here*").await;
    assert!(message.contains("nothing to import"), "got: {message}");
    assert!(
        message.contains("object(s)"),
        "should list what exists: {message}"
    );
}

#[tokio::test]
async fn ambiguous_name() {
    let pg = pg::Pg::new().unwrap();
    let table = unique_name("amb");
    pg.conn
        .execute_batch("CREATE SCHEMA IF NOT EXISTS p.other;")
        .expect("create schema");
    for schema in ["public", "other"] {
        pg.conn
            .execute_batch(&format!(
                "CREATE TABLE p.{schema}.{table} (id INTEGER PRIMARY KEY, title TEXT);"
            ))
            .expect("create table");
    }

    let message = discover_err(pg::Pg::URL, &table).await;

    for schema in ["public", "other"] {
        let _ = pg
            .conn
            .execute_batch(&format!("DROP TABLE IF EXISTS p.{schema}.{table};"));
    }
    assert!(message.contains("rename one inline"), "got: {message}");
}

#[tokio::test]
async fn key_collision() {
    let pg = pg::Pg::new().unwrap();
    let base = unique_name("col");
    pg.conn
        .execute_batch(&format!(
            "CREATE TABLE p.public.\"{base} x\" (id INTEGER PRIMARY KEY, title TEXT); \
             CREATE TABLE p.public.{base}_x (id INTEGER PRIMARY KEY, title TEXT);"
        ))
        .expect("create tables");

    let message = discover_err(pg::Pg::URL, &format!("{base}*")).await;

    for table in [format!("\"{base} x\""), format!("{base}_x")] {
        let _ = pg
            .conn
            .execute_batch(&format!("DROP TABLE IF EXISTS p.public.{table};"));
    }
    assert!(message.contains("both map to collection"), "got: {message}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn read_errors_omit_the_generated_sql(ctx: &mut Scratch) {
    let missing = format!("{}/missing.parquet", ctx.scratch().display());

    let message = discover_err(&missing, "*").await;
    assert!(message.contains("No files found"), "got: {message}");
    assert!(!message.contains("SELECT"), "got: {message}");
}

#[tokio::test]
async fn connect_errors_redact_the_password() {
    let message = discover_err("postgres://u:hunter2@127.0.0.1:1/db", "*").await;
    assert!(!message.contains("hunter2"), "got: {message}");
    assert!(message.contains("***"), "got: {message}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn all_objects_skipped_says_so(ctx: &mut Scratch) {
    let path = ctx.sql_parquet("idonly", "SELECT i AS id FROM range(3) t(i)");

    let message = discover_err(&path, "*").await;
    assert!(
        message.contains("all 1 matched object(s) were skipped"),
        "got: {message}"
    );
}
