use test_context::test_context;
use topk_protos::v1::data::Value;
use topk_protos::{doc, schema};
use topk_rs::query::{field, select};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_by_non_primitive(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("title"), 3, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(
        matches!(err, Error::InvalidArgument(s) if s == "Input to SortWithLimit must produce primitive type, not String")
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_by_non_existing(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("non_existing_field"), 3, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(matches!(
        err,
        Error::InvalidArgument(s) if s == "Input to SortWithLimit must produce primitive type, not Null"
    ));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_limit_zero(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("published_year"), 0, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(matches!(
        err,
        Error::InvalidArgument(s) if s == "Invalid argument: TopK k must be > 0"
    ));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_union_u32_and_binary(ctx: &mut ProjectTestContext) {
    // create collection
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), schema!())
        .await
        .expect("could not create collection");

    // upsert documents
    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "1", "num" => (1 as u32)),
            doc!("_id" => "11", "num" => Value::binary(vec![1, 2, 3])),
        ])
        .await
        .expect("upsert failed");

    // wait for writes to be flushed
    let _ = ctx
        .client
        .collection(&collection.name)
        .count(Some(lsn), None)
        .await
        .expect("could not query");

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("num"), 100, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(matches!(
        err,
        Error::InvalidArgument(s) if s == "Input to SortWithLimit must produce primitive type, not Union([Primitive(U32), Binary])"
    ));
}
