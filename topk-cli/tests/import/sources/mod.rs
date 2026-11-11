mod elasticsearch;
mod files;
mod mongo;
mod postgres;
mod sql;
mod topk;

use std::collections::BTreeMap;
use test_macros::rstest_ctx;

use serde_json::json;
use topk_rs::proto::v1::control::FieldSpec;

use crate::import::common::seed::Backend;
use crate::import::common::{books, json_documents, source, target, Scratch};

#[rstest_ctx(Scratch)]
#[case::sqlite(Backend::Sqlite, FieldSpec::integer(false))]
#[case::postgres(Backend::Postgres, FieldSpec::boolean(false))]
#[case::mysql(Backend::Mysql, FieldSpec::boolean(false))]
#[case::mongo(Backend::Mongo, FieldSpec::boolean(false))]
#[case::elasticsearch(Backend::Elasticsearch, FieldSpec::boolean(false))]
#[case::parquet(Backend::Parquet, FieldSpec::boolean(false))]
#[case::s3(Backend::S3, FieldSpec::boolean(false))]
async fn catalog_types(ctx: &Scratch, #[case] backend: Backend, #[case] in_print: FieldSpec) {
    let seeded = ctx.seed(backend, "books", books()).await;
    let source = source(&seeded.url).await.unwrap();
    let catalog = &source.catalog().await.unwrap().tables;
    let table = catalog
        .iter()
        .find(|table| table.from == seeded.from)
        .expect("seeded object in catalog");
    let types: BTreeMap<_, _> = table
        .columns
        .iter()
        .map(|(name, field)| (name.as_str(), field.clone().unwrap()))
        .collect();
    let mut text = FieldSpec::text(false);
    if matches!(backend, Backend::Elasticsearch) {
        text.index = Some(topk_rs::proto::v1::control::FieldIndex::keyword(
            topk_rs::proto::v1::control::KeywordIndexType::Text,
        ));
    }
    assert_eq!(
        types,
        BTreeMap::from([
            ("_id", FieldSpec::text(false)),
            ("author", text.clone()),
            ("in_print", in_print),
            ("published_year", FieldSpec::integer(false)),
            ("rating", FieldSpec::float(false)),
            ("title", text),
        ])
    );
}

#[rstest_ctx(Scratch)]
#[case::sqlite(Backend::Sqlite, FieldSpec::integer(false), json!(1))]
#[case::postgres(Backend::Postgres, FieldSpec::boolean(false), json!(true))]
#[case::mysql(Backend::Mysql, FieldSpec::boolean(false), json!(true))]
#[case::mongo(Backend::Mongo, FieldSpec::boolean(false), json!(true))]
#[case::elasticsearch(Backend::Elasticsearch, FieldSpec::boolean(false), json!(true))]
#[case::parquet(Backend::Parquet, FieldSpec::boolean(false), json!(true))]
#[case::s3(Backend::S3, FieldSpec::boolean(false), json!(true))]
async fn filters_documents(
    ctx: &Scratch,
    #[case] backend: Backend,
    #[case] bool_type: FieldSpec,
    #[case] in_print: serde_json::Value,
) {
    let seeded = ctx.seed(backend, "books", books()).await;
    let text = FieldSpec::text(false);
    let mut target = target(
        &seeded.from,
        "_id",
        [
            ("title", text.clone()),
            ("author", text),
            ("published_year", FieldSpec::integer(false)),
            ("rating", FieldSpec::float(false)),
            ("in_print", bool_type),
        ],
    );
    target.filter = Some(backend.filter().into());
    let source = source(&seeded.url).await.unwrap();
    let docs = json_documents(&source, &target).await.unwrap();
    assert_eq!(
        docs,
        [(
            None,
            json!({"_id": "mockingbird", "title": "To Kill a Mockingbird", "author": "Lee",
        "published_year": 1960, "rating": 4.3, "in_print": in_print})
        )]
    );
}

#[rstest_ctx(Scratch)]
#[case::sqlite(Backend::Sqlite)]
#[case::postgres(Backend::Postgres)]
#[case::mysql(Backend::Mysql)]
#[case::mongo(Backend::Mongo)]
#[case::elasticsearch(Backend::Elasticsearch)]
#[case::parquet(Backend::Parquet)]
#[case::s3(Backend::S3)]
async fn limits_documents(ctx: &Scratch, #[case] backend: Backend) {
    let seeded = ctx.seed(backend, "books", books()).await;
    let mut target = target(&seeded.from, "_id", [("title", FieldSpec::text(false))]);
    target.limit = Some(2);
    let source = source(&seeded.url).await.unwrap();
    let docs = json_documents(&source, &target).await.unwrap();
    assert_eq!(docs.len(), 2);
    assert_ne!(docs[0].1["_id"], docs[1].1["_id"]);
    let expected = [
        json!({"_id": "mockingbird", "title": "To Kill a Mockingbird"}),
        json!({"_id": "nineteen_eighty_four", "title": "1984"}),
        json!({"_id": "pride", "title": "Pride and Prejudice"}),
    ];
    for (partition, doc) in docs {
        assert_eq!(partition, None);
        assert!(expected.contains(&doc), "unexpected document: {doc}");
    }
}
