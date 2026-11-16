use crate::import::common::seed::pg;
use crate::import::common::*;
use sqlx::{Executor, PgPool};
use test_macros::rstest_ctx;
use topk::import::plan::Target;

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
    let ambiguous = pg::source(&base).await.unwrap();
    assert!(refused(ambiguous.discover().await).contains("ambiguous; qualify"));
    let missing = pg::source(&format!("{base}_missing")).await.unwrap();
    assert!(refused(missing.discover().await).contains("was not found"));
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
        let source = pg::source(&from).await.unwrap();
        let table = source.discover().await.unwrap();
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
    let source = source("postgres://u:hunter2@127.0.0.1:1/db#orders")
        .await
        .unwrap();
    let message = refused(source.discover().await);
    assert!(!message.contains("hunter2"), "got: {message}");
    assert!(message.contains("***"), "got: {message}");
}

#[rstest_ctx(Scratch)]
async fn selected_table_only_permissions(ctx: &Scratch) {
    let name = ctx.name("selected");
    let other = ctx.name("private");
    let role = ctx.name("reader");
    let _tables = pg::Pg::temp(&[
        (
            format!("public.{name}"),
            "(id INTEGER PRIMARY KEY, title TEXT)",
        ),
        (
            format!("public.{other}"),
            "(id INTEGER PRIMARY KEY, title TEXT)",
        ),
    ]);
    let admin = PgPool::connect(pg::Pg::URL).await.unwrap();
    admin.execute(format!("CREATE ROLE {role} LOGIN PASSWORD 'reader'; GRANT USAGE ON SCHEMA public TO {role}; GRANT SELECT ON public.{name} TO {role}; INSERT INTO public.{name} VALUES (1, 'selected')").as_str()).await.unwrap();
    let restricted = format!("postgres://{role}:reader@localhost:5433/demo?sslmode=disable");
    let conn = PgPool::connect(&restricted).await.unwrap();
    assert!(conn
        .execute(format!("SELECT * FROM public.{other}").as_str())
        .await
        .is_err());
    let selected = source(&format!("{restricted}#{name}")).await.unwrap();
    let target = Target::new(selected.discover().await.unwrap(), None).unwrap();
    assert_eq!(target.from.value(), format!("public.{name}"));
    assert_eq!(json_documents(&selected, &target).await.unwrap().len(), 1);
    conn.close().await;
    admin
        .execute(format!("DROP OWNED BY {role}; DROP ROLE {role}").as_str())
        .await
        .unwrap();
}
