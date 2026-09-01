use crate::common::seed::{self as seed, es, mongo, mysql, parquet, pg, sqlite, xlsx, Seed};
use crate::common::*;
use indexmap::IndexMap;
use rstest::rstest;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::{Field, Target};

fn book_fields() -> IndexMap<String, Field> {
    fields_toml(
        r#"title = { type = "text", index = "keyword" }
           rating = { type = "float" }
           in_print = { type = "bool" }"#,
    )
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

#[rstest]
#[case::postgres("postgres://", Box::new(pg::Pg::new().unwrap()), vec![("PGHOST", "localhost"), ("PGPORT", "5433"), ("PGUSER", "postgres"), ("PGPASSWORD", "postgres"), ("PGDATABASE", "demo")])]
#[case::mongo("mongodb://", Box::new(mongo::Mongo::client()), vec![("MONGODB_URI", mongo::Mongo::URL)])]
#[tokio::test]
async fn a_bare_scheme_connects_from_the_environment(
    #[case] scheme: &str,
    #[case] backend: Box<dyn Seed>,
    #[case] env: Vec<(&str, &str)>,
) {
    let (object, _) = seeded_books(&*backend).await;
    let out = ok(&["import", scheme, &object.from, "--dry-run"], &env);
    assert!(
        out.contains(&object.from),
        "bare {scheme} didn't discover {}:\n{out}",
        object.from
    );
}

/// One filter per source language: SQL for files and databases, query DSL for
/// mongodb and elasticsearch.
#[rstest]
#[case::sql(Box::new(parquet::File::new().unwrap()), "published_year > 1950")]
#[case::mongo(
    Box::new(mongo::Mongo::client()),
    r#"{"published_year": {"$gt": 1950}}"#
)]
#[case::elasticsearch(
    Box::new(es::Es::client()),
    r#"{"range": {"published_year": {"gt": 1950}}}"#
)]
#[tokio::test]
async fn a_filter_selects_matching_rows(#[case] backend: Box<dyn Seed>, #[case] filter: &str) {
    let (object, url) = seeded_books(&*backend).await;
    let target = Target {
        filter: Some(filter.to_string()),
        ..object
    };
    let docs = stream_docs_from(url.as_deref(), &target).await.unwrap();
    assert_eq!(docs.keys().cloned().collect::<Vec<_>>(), ["mockingbird"]);
}

#[rstest]
#[case::parquet(Box::new(parquet::File::new().unwrap()))]
#[case::sqlite(Box::new(sqlite::Db::new().unwrap()))]
#[case::postgres(Box::new(pg::Pg::new().unwrap()))]
#[case::mysql(Box::new(mysql::MySql::new().unwrap()))]
#[case::mongo(Box::new(mongo::Mongo::client()))]
#[case::elasticsearch(Box::new(es::Es::client()))]
#[tokio::test]
async fn a_limit_caps_rows(#[case] backend: Box<dyn Seed>) {
    let (object, url) = seeded_books(&*backend).await;
    let target = Target {
        limit: Some(2),
        ..object
    };
    let docs = stream_docs_from(url.as_deref(), &target).await.unwrap();
    assert_eq!(docs.len(), 2);
}

/// A query elasticsearch rejects comes back with elasticsearch's reason, not
/// just the status line — `error_for_status_code` throws that body away.
#[tokio::test]
async fn es_rejected_query_carries_the_reason() {
    let (object, url) = seeded_books(&es::Es::client()).await;
    let target = Target {
        filter: Some(r#"{"bogus_query": {}}"#.to_string()),
        ..object
    };
    let error = refused(stream_docs_from(url.as_deref(), &target).await);
    // The status line does not name the query; only the body does.
    assert!(error.contains("400"), "got: {error}");
    assert!(error.contains("bogus_query"), "got: {error}");
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
