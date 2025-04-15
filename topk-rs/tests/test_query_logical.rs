use test_context::test_context;
use topk_protos::v1::data::{stage::filter_stage::FilterExpr, LogicalExpr, Query, Stage};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_lte(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::filter(FilterExpr::logical(LogicalExpr::lte(
                    LogicalExpr::field("published_year"),
                    LogicalExpr::literal((1950 as u32).into()),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984", "pride", "hobbit", "moby", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_and(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::filter(FilterExpr::logical(LogicalExpr::and(
                    LogicalExpr::lte(
                        LogicalExpr::field("published_year"),
                        LogicalExpr::literal((1950 as u32).into()),
                    ),
                    LogicalExpr::gte(
                        LogicalExpr::field("published_year"),
                        LogicalExpr::literal((1948 as u32).into()),
                    ),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984"]);
}
