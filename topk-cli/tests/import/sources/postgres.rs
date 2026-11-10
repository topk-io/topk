use topk::import::discover;

use crate::import::common::seed::pg;
use crate::import::common::*;

#[tokio::test]
async fn catalog_selection() {
    let base = unique_name("catalog");
    let keyed = format!("{base}_keyed");
    let composite = format!("{base}_composite");
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
        (format!("public.{base}"), columns),
        (format!("other.{base}"), columns),
        (format!("public.\"{base} x\""), columns),
        (format!("public.{base}_x"), columns),
    ]);
    let catalog = catalog_of(pg::Pg::URL).await.unwrap();
    for (name, id) in [(&keyed, "sku"), (&composite, "<column>")] {
        let spec = discover(&catalog, &[format!("public.{name}")], None, None).unwrap();
        assert_eq!(spec.collections.keys().collect::<Vec<_>>(), [name]);
        assert_eq!(spec.collections[name].id.as_deref(), Some(id));
    }
    let renamed = discover(&catalog, &[format!("public.{keyed}=renamed")], None, None).unwrap();
    assert_eq!(renamed.collections.keys().collect::<Vec<_>>(), ["renamed"]);
    assert_eq!(
        renamed.collections["renamed"].from,
        format!("public.{keyed}")
    );
    let matched = discover(&catalog, &[format!("public.{base}_key*")], None, None).unwrap();
    assert_eq!(matched.collections.keys().collect::<Vec<_>>(), [&keyed]);
    for (pattern, fragment) in [
        (format!("{base}_missing*"), "nothing to import"),
        (format!("{base}_missing*"), "object(s)"),
        (base.clone(), "rename one inline"),
        (format!("public.{base}?x"), "both map to collection"),
    ] {
        let message = refused(discover(&catalog, &[pattern], None, None));
        assert!(message.contains(fragment), "{message}");
    }
}

#[tokio::test]
async fn connect_errors_redact_the_password() {
    let message = discover_err("postgres://u:hunter2@127.0.0.1:1/db", "*").await;
    assert!(!message.contains("hunter2"), "got: {message}");
    assert!(message.contains("***"), "got: {message}");
}
