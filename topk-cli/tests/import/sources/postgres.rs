use crate::import::common::seed::pg;
use crate::import::common::*;
use test_macros::rstest_ctx;

#[rstest_ctx(Scratch)]
async fn catalog_metadata(ctx: &Scratch) {
    let base = ctx.name("catalog");
    let keyed = format!("{base}_keyed");
    let composite = format!("{base}_composite");
    let unique = format!("{base}_unique");
    let nullable = format!("{base}_nullable");
    pg::Pg::new()
        .unwrap()
        .conn
        .execute_batch("CREATE SCHEMA IF NOT EXISTS p.other;")
        .expect("create schema");
    let columns = "(id INTEGER PRIMARY KEY, title TEXT)";
    let _tables = pg::Pg::temp(&[
        (
            format!("public.{keyed}"),
            "(sku TEXT PRIMARY KEY, title TEXT)",
        ),
        (
            format!("public.{composite}"),
            "(a TEXT, b INTEGER, title TEXT, PRIMARY KEY (a, b))",
        ),
        (
            format!("public.{unique}"),
            "(sku TEXT NOT NULL UNIQUE, title TEXT)",
        ),
        (
            format!("public.{nullable}"),
            "(sku TEXT UNIQUE, title TEXT)",
        ),
        (format!("public.{base}"), columns),
        (format!("other.{base}"), columns),
        (format!("public.\"{base} x\""), columns),
        (format!("public.{base}_x"), columns),
    ]);
    let source = source(pg::Pg::URL).await.unwrap();
    let catalog = &source.catalog().await.unwrap().tables;
    for (schema, name, key, columns) in [
        ("public", keyed.as_str(), Some("sku"), vec!["sku", "title"]),
        ("public", unique.as_str(), Some("sku"), vec!["sku", "title"]),
        ("public", nullable.as_str(), None, vec!["sku", "title"]),
        ("public", composite.as_str(), None, vec!["a", "b", "title"]),
        ("public", base.as_str(), Some("id"), vec!["id", "title"]),
        ("other", base.as_str(), Some("id"), vec!["id", "title"]),
        (
            "public",
            &format!("{base} x"),
            Some("id"),
            vec!["id", "title"],
        ),
        (
            "public",
            &format!("{base}_x"),
            Some("id"),
            vec!["id", "title"],
        ),
    ] {
        let from = format!("{schema}.{name}");
        let table = catalog
            .iter()
            .find(|table| table.from == from)
            .expect(&from);
        assert_eq!(table.collection_hint.as_deref(), Some(name), "{from}");
        assert_eq!(table.key.as_deref(), key, "{from}");
        assert_eq!(
            table
                .columns
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            columns,
            "{from}"
        );
    }
}

#[tokio::test]
async fn connect_errors_redact_the_password() {
    let source = source("postgres://u:hunter2@127.0.0.1:1/db").await.unwrap();
    let message = refused(source.catalog().await);
    assert!(!message.contains("hunter2"), "got: {message}");
    assert!(message.contains("***"), "got: {message}");
}
