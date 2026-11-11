use crate::import::common::seed::Backend::Sqlite;
use crate::import::common::{books, json_documents, refused, source, target, Scratch};
use futures::TryStreamExt;
use serde_json::json;
use sqlx::{Executor, SqlitePool};
use test_macros::rstest_ctx;
use topk::import::source::{Checkpoint, Cursor, ReadEvent};
use topk_rs::proto::v1::control::FieldSpec;

#[rstest_ctx(Scratch)]
async fn unqualified_table(ctx: &Scratch) {
    let seeded = ctx.seed(Sqlite, "books", books()).await;
    let source = source(&seeded.url).await.unwrap();
    let target = target(
        seeded.from.rsplit('.').next().unwrap(),
        "_id",
        [("title", FieldSpec::text(false))],
    );
    let docs = json_documents(&source, &target).await.unwrap();
    assert_eq!(docs.len(), 3);
    assert!(docs
        .iter()
        .any(|(_, doc)| doc == &json!({"_id": "mockingbird", "title": "To Kill a Mockingbird"})));
}

#[rstest_ctx(Scratch)]
#[case::dotted_column("user.id")]
#[case::quoted_column("user\"id")]
async fn quoted_columns(ctx: &Scratch, #[case] column: &str) {
    let path = ctx.file("source.db", "");
    let pool = SqlitePool::connect(&format!("sqlite://{path}"))
        .await
        .unwrap();
    pool.execute(format!("CREATE TABLE records (\"{}\" TEXT NOT NULL PRIMARY KEY, title TEXT); INSERT INTO records VALUES ('one', 'book')", column.replace('"', "\"\"")).as_str()).await.unwrap();
    pool.close().await;
    let source = source(&format!("sqlite:{path}")).await.unwrap();
    let target = target("records", column, [("title", FieldSpec::text(false))]);
    assert_eq!(
        json_documents(&source, &target).await.unwrap(),
        [(None, json!({"_id": "one", "title": "book"}))]
    );
}

#[rstest_ctx(Scratch)]
async fn dotted_table_rejected(ctx: &Scratch) {
    let path = ctx.file("source.db", "");
    let pool = SqlitePool::connect(&format!("sqlite://{path}"))
        .await
        .unwrap();
    pool.execute("CREATE TABLE \"a.b\" (id TEXT NOT NULL PRIMARY KEY, title TEXT)")
        .await
        .unwrap();
    pool.close().await;
    let source = source(&format!("sqlite:{path}")).await.unwrap();
    assert!(refused(source.catalog().await).contains("dots inside schema or table names"));
}

#[rstest_ctx(Scratch)]
#[case::primary_key("id INTEGER PRIMARY KEY", Some("id"))]
#[case::unique_key("id TEXT NOT NULL UNIQUE", Some("id"))]
#[case::nullable_unique("id TEXT UNIQUE", None)]
#[case::nullable_primary("id TEXT PRIMARY KEY", None)]
#[case::no_key("id INTEGER", None)]
#[case::nullable_integer_primary("id INTEGER PRIMARY KEY DESC", None)]
#[case::composite_key("id INTEGER, other INTEGER, PRIMARY KEY (id, other)", None)]
async fn requires_unique_key(ctx: &Scratch, #[case] columns: &str, #[case] key: Option<&str>) {
    let path = ctx.file("keys.db", "");
    let pool = SqlitePool::connect(&format!("sqlite://{path}"))
        .await
        .unwrap();
    pool.execute(format!("CREATE TABLE records (title TEXT, {columns})").as_str())
        .await
        .unwrap();
    pool.close().await;
    let source = source(&format!("sqlite:{path}")).await.unwrap();
    let catalog = source.catalog().await.unwrap();
    assert_eq!(catalog.table("records").unwrap().key.as_deref(), key);
    let target = target("records", "id", [("title", FieldSpec::text(false))]);
    let scan = source.scan(&target, None).await;
    match key {
        Some(_) => assert!(scan.is_ok()),
        None => assert!(refused(scan).contains("single-column non-null primary or unique key")),
    }
}

#[rstest_ctx(Scratch)]
async fn resume_uses_source_key(ctx: &Scratch) {
    let path = ctx.file("resume.db", "");
    let pool = SqlitePool::connect(&format!("sqlite://{path}"))
        .await
        .unwrap();
    pool.execute("CREATE TABLE records (seq INTEGER PRIMARY KEY, external_id TEXT, title TEXT); INSERT INTO records VALUES (1, 'same', 'a'), (2, 'same', 'b'), (3, 'same', 'c')").await.unwrap();
    pool.close().await;
    let source = source(&format!("sqlite:{path}")).await.unwrap();
    let target = target(
        "records",
        "external_id",
        [("title", FieldSpec::text(false))],
    );
    let checkpoint = Checkpoint {
        cursor: Cursor::Sql {
            column: "seq".into(),
            value: "1".into(),
        },
        consumed: 1,
    };
    let mut scan = source.scan(&target, Some(&checkpoint)).await.unwrap();
    let mut documents = Vec::new();
    let mut last = None;
    while let Some(event) = scan.try_next().await.unwrap() {
        match event {
            ReadEvent::Chunk(chunk) => {
                last = chunk.cursor;
                for row in chunk.rows {
                    let (_, document) = target.document(row.unwrap()).unwrap();
                    documents.push(serde_json::Value::Object(crate::import::common::json(
                        &document,
                    )));
                }
            }
            ReadEvent::Restarted => panic!("unexpected restart"),
        }
    }
    assert_eq!(
        documents,
        [
            json!({"_id":"same", "title":"b"}),
            json!({"_id":"same", "title":"c"})
        ]
    );
    assert_eq!(
        last,
        Some(Cursor::Sql {
            column: "seq".into(),
            value: "3".into()
        })
    );
    let legacy = Checkpoint {
        cursor: Cursor::Key("same".into()),
        consumed: 1,
    };
    assert!(
        refused(source.scan(&target, Some(&legacy)).await).contains("does not match source key")
    );
}
