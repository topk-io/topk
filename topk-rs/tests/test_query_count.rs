use test_context::test_context;
use topk_protos::{
    doc,
    v1::data::{stage::filter_stage::FilterExpr, LogicalExpr, Query, Stage},
};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .query(Query::new(vec![Stage::count()]), None, None)
        .await
        .expect_err("should not be able to query non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_count(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(Query::new(vec![Stage::count()]), None, None)
        .await
        .expect("could not query");

    assert_eq!(result, vec![doc!("_count" => 10_u64)]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_count_with_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let query = Query::new(vec![
        Stage::filter(FilterExpr::logical(LogicalExpr::lte(
            LogicalExpr::field("published_year"),
            LogicalExpr::literal((1950 as u32).into()),
        ))),
        Stage::count(),
    ]);

    let result = ctx
        .client
        .collection(&collection.name)
        .query(query, None, None)
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
        .query(Query::new(vec![Stage::count()]), None, None)
        .await
        .expect("could not query");

    assert_eq!(result, vec![doc!("_count" => 10_u64)]);

    let lsn = ctx
        .client
        .collection(&collection.name)
        .delete(vec!["lotr".to_string()])
        .await
        .expect("could not delete document");

    let result = ctx
        .client
        .collection(&collection.name)
        .query(Query::new(vec![Stage::count()]), Some(lsn), None)
        .await
        .expect("could not query");

    assert_eq!(result, vec![doc!("_count" => 9_u64)]);
}
