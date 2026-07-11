use test_context::test_context;

use topk_rs::proto::v1::data::stage::sort_stage::SortOrder;
use topk_rs::query::{field, select};

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
