use test_context::test_context;
use topk_rs::query::field;
use topk_rs::query::filter;

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
            filter(field("_id").starts_with("cat")).limit(100),
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
        .query(filter(field("_id").starts_with("")).limit(100), None, None)
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
            filter(field("_id").starts_with("foobarbaz")).limit(100),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(result.is_empty());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_starts_with_list_string_scalar(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("tags").starts_with("lov")).limit(100),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_starts_with_list_string_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("tags").starts_with(field("_id"))).limit(100),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "hobbit"]);
}
