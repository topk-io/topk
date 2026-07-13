use test_context::test_context;

use topk_rs::data::literal;
use topk_rs::proto::v1::data::stage::sort_stage::SortOrder;
use topk_rs::query::{field, select};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sort_by_scalar(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))])
                .sort("published_year")
                .limit(3),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 3);
    assert_fields!(&result, ["_id"]);
    assert_doc_ids_ordered!(result, ["harry", "alchemist", "mockingbird"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sort_by_multiple_fields(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))])
                .sort([
                    (field("rating"), SortOrder::Desc),
                    (field("published_year"), SortOrder::Asc),
                ])
                .limit(4),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids_ordered!(result, ["moby", "pride", "gatsby", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sort_by_multiple_with_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))])
                .sort([
                    (literal(1u32).into(), SortOrder::Desc),
                    (field("published_year"), SortOrder::Asc),
                ])
                .limit(4),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids_ordered!(result, ["pride", "moby", "gatsby", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sort_by_too_many_fields(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))])
                .sort(
                    (0..10)
                        .map(|i| (field(format!("field_{i}")), SortOrder::Asc))
                        .collect::<Vec<_>>(),
                )
                .limit(4),
            None,
            None,
        )
        .await
        .expect_err("Query should have failed validation");

    assert!(matches!(
        err,
        Error::InvalidArgument(s) if s.contains("Sort must have at most 8 expressions")
    ));
}
