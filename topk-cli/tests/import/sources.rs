use crate::common::seed::{self as seed, es, mongo, mysql, parquet, pg, sqlite, xlsx, Seed};
use crate::common::*;
use indexmap::IndexMap;
use serde_json::json;
use test_context::test_context;
use test_macros::rstest_ctx;
use topk::import::{Field, Index, Target, Type};

fn book_fields() -> IndexMap<String, Field> {
    fields([
        (
            "title",
            Field {
                ty: Type::Text,
                index: Some(Index::Keyword),
                ..Default::default()
            },
        ),
        (
            "rating",
            Field {
                ty: Type::Float,
                ..Default::default()
            },
        ),
        (
            "in_print",
            Field {
                ty: Type::Bool,
                ..Default::default()
            },
        ),
    ])
}

#[rstest_ctx(Ctx)]
#[case::sqlite(Box::new(sqlite::Db::new().unwrap()))]
#[case::postgres(Box::new(pg::Pg::new().unwrap()))]
#[case::mysql(Box::new(mysql::MySql::new().unwrap()))]
#[case::mongo(Box::new(mongo::Mongo::client()))]
#[case::elasticsearch(Box::new(es::Es::client()))]
#[case::parquet(Box::new(parquet::File::new().unwrap()))]
#[case::xlsx(Box::new(xlsx::File::new().unwrap()))]
async fn roundtrip(ctx: &mut Ctx, #[case] backend: Box<dyn Seed>) {
    let name = unique_name("books");
    let object = backend.seed(&name, books()).await.unwrap();
    let url = backend.url();

    let preview = ctx.target_spec(
        "preview",
        Target {
            fields: book_fields(),
            ..object.clone()
        },
    );
    let docs = dry_run_from(url.as_deref(), &preview, &[]);
    assert_eq!(docs["mockingbird"]["title"], json!("To Kill a Mockingbird"));
    assert_eq!(docs["mockingbird"]["rating"], json!(4.3));
    assert_eq!(docs["pride"]["in_print"], json!(false));

    let collection = ctx.collection("books");
    let spec = ctx.target_spec(
        &collection,
        Target {
            fields: book_fields(),
            ..object
        },
    );
    ok(&import_args(url.as_deref(), &spec, &["--yes"]), &[]);
    let got = ctx.get(&collection, &["mockingbird", "pride"]).await;
    assert_eq!(
        field(&got["mockingbird"], "title"),
        json!("To Kill a Mockingbird")
    );
    assert_eq!(field(&got["mockingbird"], "rating"), json!(4.3));
    assert_eq!(field(&got["pride"], "in_print"), json!(false));
}

#[test]
fn pg_env() {
    let table = pg::Pg::seed();
    let out = ok(
        &["import", "postgres://", &table, "--dry-run"],
        &[
            ("PGHOST", "localhost"),
            ("PGPORT", "5433"),
            ("PGUSER", "postgres"),
            ("PGPASSWORD", "postgres"),
            ("PGDATABASE", "demo"),
        ],
    );
    assert!(
        out.contains(&table),
        "bare postgres:// didn't discover {table}:\n{out}"
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn sql_filter(ctx: &mut Scratch) {
    let object = ctx.seed_parquet("books", books()).await;
    let target = Target {
        filter: Some("published_year > 1950".to_string()),
        ..object
    };
    let docs = stream_docs(&target).await.unwrap();
    assert_eq!(docs.keys().cloned().collect::<Vec<_>>(), ["mockingbird"]);
}

#[tokio::test]
async fn mongo_env() {
    let client = mongo::Mongo::client();
    let object = client.seed(&unique_name("books"), books()).await.unwrap();
    let out = ok(
        &["import", "mongodb://", &object.from, "--dry-run"],
        &[("MONGODB_URI", mongo::Mongo::URL)],
    );
    assert!(
        out.contains(&object.from),
        "bare mongodb:// didn't discover {}:\n{out}",
        object.from
    );
}

#[tokio::test]
async fn mongo_filter() {
    let client = mongo::Mongo::client();
    let object = client.seed(&unique_name("books"), books()).await.unwrap();
    let target = Target {
        filter: Some(r#"{"published_year": {"$gt": 1950}}"#.to_string()),
        ..object
    };
    let docs = stream_docs_from(client.url().as_deref(), &target)
        .await
        .unwrap();
    assert_eq!(docs.keys().cloned().collect::<Vec<_>>(), ["mockingbird"]);
}

#[tokio::test]
async fn es_filter() {
    let client = es::Es::client();
    let object = client.seed(&unique_name("books"), books()).await.unwrap();
    let target = Target {
        filter: Some(r#"{"range": {"published_year": {"gt": 1950}}}"#.to_string()),
        ..object
    };
    let docs = stream_docs_from(client.url().as_deref(), &target)
        .await
        .unwrap();
    assert_eq!(docs.keys().cloned().collect::<Vec<_>>(), ["mockingbird"]);
}

/// A query elasticsearch rejects comes back with elasticsearch's reason, not
/// just the status line — `error_for_status_code` throws that body away.
#[tokio::test]
async fn es_rejected_query_carries_the_reason() {
    let client = es::Es::client();
    let object = client.seed(&unique_name("books"), books()).await.unwrap();
    let target = Target {
        filter: Some(r#"{"bogus_query": {}}"#.to_string()),
        ..object
    };
    let error = stream_docs_from(client.url().as_deref(), &target)
        .await
        .expect_err("elasticsearch refuses the query")
        .to_string();
    // The status line does not name the query; only the body does.
    assert!(error.contains("400"), "got: {error}");
    assert!(error.contains("bogus_query"), "got: {error}");
}

#[tokio::test]
async fn mongo_limit() {
    let client = mongo::Mongo::client();
    let object = client.seed(&unique_name("books"), books()).await.unwrap();
    let target = Target {
        limit: Some(2),
        ..object
    };
    let docs = stream_docs_from(client.url().as_deref(), &target)
        .await
        .unwrap();
    assert_eq!(docs.len(), 2);
}

#[tokio::test]
async fn es_limit() {
    let client = es::Es::client();
    let object = client.seed(&unique_name("books"), books()).await.unwrap();
    let target = Target {
        limit: Some(2),
        ..object
    };
    let docs = stream_docs_from(client.url().as_deref(), &target)
        .await
        .unwrap();
    assert_eq!(docs.len(), 2);
}

#[tokio::test]
async fn mongo_binary() {
    let name = unique_name("bson");
    let client = mongo::Mongo::client();
    client
        .database("demo")
        .collection::<mongodb::bson::Document>(&name)
        .insert_one(mongodb::bson::doc! {
            "_id": "b1",
            "bin": mongodb::bson::Binary {
                subtype: mongodb::bson::spec::BinarySubtype::Generic,
                bytes: vec![0xde, 0xad, 0xbe, 0xef],
            },
        })
        .await
        .unwrap();

    let target = seed::discovered(
        Target {
            from: name,
            ..Default::default()
        },
        Some(mongo::Mongo::URL.to_string()),
    )
    .await
    .unwrap();
    let docs = stream_docs_from(Some(mongo::Mongo::URL), &target)
        .await
        .unwrap();
    assert_eq!(
        docs["b1"],
        json!({"_id": "b1", "bin": [222, 173, 190, 239]})
    );
}
