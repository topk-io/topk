use crate::import::common::*;
use test_context::test_context;
use topk::import::discover;

#[test_context(Scratch)]
#[tokio::test]
async fn inferred_types(ctx: &mut Scratch) {
    let source = ctx.parquet(
        "SELECT 1 AS id, [0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8]::FLOAT[] AS embedding, \
         DATE '2024-01-15' AS d, TIMESTAMP '2024-06-30 12:00:00' AS ts, \
         {'indices': [3,1]::UINTEGER[], 'values': [1.5,0.5]::FLOAT[]} AS sv",
    );

    let catalog = catalog_of(&source).await.unwrap();
    let spec = discover(&catalog, &[], None, None).unwrap();
    let target = spec.collections.values().next().unwrap();

    assert_eq!(target.fields["embedding"].ty.to_string(), "f32_vector");
    assert_eq!(target.fields["embedding"].dim, Some(8));
    assert_eq!(target.fields["d"].ty.to_string(), "timestamp");
    assert_eq!(target.fields["ts"].ty.to_string(), "timestamp");
    assert_eq!(target.fields["sv"].ty.to_string(), "f32_sparse_vector");
}

#[test_context(Scratch)]
#[tokio::test]
async fn ragged_float_lists_stay_lists(ctx: &mut Scratch) {
    let source = ctx.parquet(
        "SELECT i AS id, CASE WHEN i = 0 THEN [0.1,0.2] ELSE [0.3] END::FLOAT[] AS embedding \
         FROM range(3) t(i)",
    );
    let catalog = catalog_of(&source).await.unwrap();
    let spec = discover(&catalog, &[], None, None).unwrap();
    let field = &spec.collections.values().next().unwrap().fields["embedding"];
    assert_eq!(field.ty.to_string(), "float_list");
    assert_eq!(field.dim, None);
}

/// Field names may not start with `_`, but sources use that for their own
/// bookkeeping columns — so a discovered spec renames them and runs as written.
#[test_context(Scratch)]
#[tokio::test]
async fn underscore_columns_are_renamed(ctx: &mut Scratch) {
    let source =
        ctx.parquet("SELECT i AS _id, 'x' AS _lang, 1 AS _n_chars, 'y' AS text FROM range(2) t(i)");

    let catalog = catalog_of(&source).await.unwrap();
    let spec = discover(&catalog, &[], None, None).unwrap();
    let target = spec.collections.values().next().unwrap();

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
    let source = ctx.parquet("SELECT i AS _id, 'a' AS _lang, 'b' AS lang FROM range(2) t(i)");
    let catalog = catalog_of(&source).await.unwrap();
    let error = refused(discover(&catalog, &[], None, None));

    assert!(
        error.contains("\"_lang\": field names cannot be empty or start with `_`"),
        "{error}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn all_objects_skipped_says_so(ctx: &mut Scratch) {
    let source = ctx.parquet("SELECT i AS id FROM range(3) t(i)");
    let catalog = catalog_of(&source).await.unwrap();
    let error = refused(discover(&catalog, &[], None, None));

    assert!(
        error.contains("all 1 matched object(s) were skipped"),
        "{error}"
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

    let catalog = catalog_of(&pattern).await.unwrap();
    let spec = topk::import::discover(&catalog, &[], Some("parts"), None)
        .expect("--to names the collection");
    assert_eq!(spec.collections.keys().collect::<Vec<_>>(), ["parts"]);
}

#[test_context(Scratch)]
#[tokio::test]
async fn read_errors_name_the_source(ctx: &mut Scratch) {
    let missing = format!("{}/missing.parquet", ctx.scratch().display());

    let message = discover_err(&missing, "*").await;
    assert!(
        message.contains("reading") && message.contains("missing.parquet"),
        "got: {message}"
    );
    assert!(message.contains("No files found"), "got: {message}");
}
