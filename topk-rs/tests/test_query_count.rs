use test_context::test_context;
use topk_rs::doc;
use topk_rs::query::{field, filter};
use topk_rs::Error;
use topk_rs::schema;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .count(None, None)
        .await
        .expect_err("should not be able to query non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_count_empty_collection(ctx: &mut ProjectTestContext) {
    let collection: topk_rs::proto::v1::control::Collection = ctx
        .client
        .collections()
        .create(ctx.wrap("empty"), schema!())
        .await
        .expect("could not create collection");

    let count  = ctx
        .client
        .collection(collection.name)
        .count(None, Some(topk_rs::proto::v1::data::ConsistencyLevel::Strong))
        .await
        .expect("could not query");

    assert_eq!(count, 0)
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_count(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .count(None, None)
        .await
        .expect("could not query");

    assert_eq!(result, 10_u64);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_count_with_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("published_year").lte(1950 as u32)).count(),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result, vec![doc!("_count" => 5_u64)]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_count_with_delete(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .count(None, None)
        .await
        .expect("could not query");

    assert_eq!(result, 10_u64);

    let lsn = ctx
        .client
        .collection(&collection.name)
        .delete(vec!["lotr".to_string()])
        .await
        .expect("could not delete document");

    let result = ctx
        .client
        .collection(&collection.name)
        .count(Some(lsn), None)
        .await
        .expect("could not query");

    assert_eq!(result, 9_u64);
}
