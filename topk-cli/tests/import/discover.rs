use crate::common::seed::pg;
use crate::common::*;
use test_context::test_context;
use topk::import::{Error, Uri};

async fn catalog_of(locator: &str) -> Vec<topk::import::Table> {
    let uri: Uri = locator.parse().expect("source uri parses");
    topk::import::Source::connect(&uri, &topk::endpoint::Endpoint::default())
        .await
        .expect("connect")
        .catalog()
        .await
        .expect("catalog")
}

async fn discover_err(locator: &str, pattern: &str) -> String {
    let uri: Uri = locator.parse().expect("source uri parses");
    let result = async {
        let catalog = topk::import::Source::connect(&uri, &topk::endpoint::Endpoint::default())
            .await?
            .catalog()
            .await?;
        topk::import::discover(&catalog, &[pattern.to_string()], None, None)
    }
    .await;
    match result {
        Err(Error::InvalidArgument(message)) => message,
        Err(other) => panic!("expected InvalidArgument, got {other:?}"),
        Ok(discovered) => panic!(
            "expected discover to fail, got {} collection(s)",
            discovered.collections.len()
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
async fn date_and_timestamp_columns_are_timestamps(ctx: &mut Scratch) {
    let path = ctx.sql_parquet(
        "when",
        "SELECT i AS id, DATE '2024-01-15' AS d, TIMESTAMP '2024-06-30 12:00:00' AS ts \
         FROM range(2) t(i)",
    );

    let spec = discover_spec(&path, None).await;
    let fields = &spec.collections["when"].fields;
    assert_eq!(fields["d"].ty.to_string(), "timestamp");
    assert_eq!(fields["ts"].ty.to_string(), "timestamp");
}

/// Field names may not start with `_`, but sources use that for their own
/// bookkeeping columns — so a discovered spec renames them and runs as written.
#[test_context(Scratch)]
#[tokio::test]
async fn underscore_columns_are_renamed(ctx: &mut Scratch) {
    let path = ctx.sql_parquet(
        "tdf",
        "SELECT i AS _id, 'x' AS _lang, 1 AS _n_chars, 'y' AS text FROM range(2) t(i)",
    );

    let spec = discover_spec(&path, None).await;
    let target = &spec.collections["tdf"];
    assert_eq!(target.id.as_deref(), Some("_id"));
    assert_eq!(target.fields["lang"].from.as_deref(), Some("_lang"));
    assert_eq!(target.fields["n_chars"].from.as_deref(), Some("_n_chars"));
    // A column that never needed renaming keeps its name and carries no `from`.
    assert_eq!(target.fields["text"].from, None);
    assert!(!target.fields.contains_key("_lang"));
}

/// A stripped name that collides is left alone, so validation rejects it: a
/// clear error beats silently folding two columns onto one field.
#[test_context(Scratch)]
#[tokio::test]
async fn underscore_rename_yields_to_a_collision(ctx: &mut Scratch) {
    let path = ctx.sql_parquet(
        "clash",
        "SELECT i AS _id, 'a' AS _lang, 'b' AS lang FROM range(2) t(i)",
    );

    let catalog = catalog_of(&path).await;
    let message = refused(topk::import::discover(&catalog, &[], None, None));
    assert!(
        message.contains("\"_lang\": field names cannot be empty or start with `_`"),
        "got: {message}"
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

    let catalog = catalog_of(&pattern).await;
    let spec = topk::import::discover(&catalog, &[], Some("parts"), None)
        .expect("--to names the collection");
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), ["parts"]);
}

#[tokio::test]
async fn inline_rename() {
    let table = pg::Pg::seed_keyed_on("sku");
    let catalog = catalog_of(pg::Pg::URL).await;
    let spec = topk::import::discover(&catalog, &[format!("{table}=renamed")], None, None)
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
    let table = unique_name("amb");
    pg::Pg::new()
        .unwrap()
        .conn
        .execute_batch("CREATE SCHEMA IF NOT EXISTS p.other;")
        .expect("create schema");
    let columns = "(id INTEGER PRIMARY KEY, title TEXT)";
    let _tables = pg::Pg::temp(&[
        (format!("public.{table}"), columns),
        (format!("other.{table}"), columns),
    ]);

    let message = discover_err(pg::Pg::URL, &table).await;
    assert!(message.contains("rename one inline"), "got: {message}");
}

#[tokio::test]
async fn key_collision() {
    let base = unique_name("col");
    let columns = "(id INTEGER PRIMARY KEY, title TEXT)";
    let _tables = pg::Pg::temp(&[
        (format!("public.\"{base} x\""), columns),
        (format!("public.{base}_x"), columns),
    ]);

    let message = discover_err(pg::Pg::URL, &format!("{base}*")).await;
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
