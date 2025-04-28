use test_context::test_context;
use topk_rs::query::field;
use topk_rs::query::filter;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_contains(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").contains("atch")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["catcher"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_contains_no_match(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").contains("rubbish")).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, Vec::<String>::new());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_contains_empty(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").contains("")).topk(field("published_year"), 100, false),
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
