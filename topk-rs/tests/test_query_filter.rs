use test_context::test_context;
use topk_protos::v1::data::{stage::filter_stage::FilterExpr, LogicalExpr, Query, Stage};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_starts_with(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::filter(FilterExpr::logical(LogicalExpr::starts_with(
                    LogicalExpr::field("_id"),
                    LogicalExpr::literal("cat".into()),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, false),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["catcher"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_starts_with_empty(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::filter(FilterExpr::logical(LogicalExpr::starts_with(
                    LogicalExpr::field("_id"),
                    LogicalExpr::literal("".into()),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, false),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(
        result,
        [
            "gatsby",
            "catcher",
            "moby",
            "mockingbird",
            "alchemist",
            "harry",
            "lotr",
            "pride",
            "1984",
            "hobbit"
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_starts_with_non_existent_prefix(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::filter(FilterExpr::logical(LogicalExpr::starts_with(
                    LogicalExpr::field("_id"),
                    LogicalExpr::literal("foobarbaz".into()),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, false),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(result.is_empty());
}
