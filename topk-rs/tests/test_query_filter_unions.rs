use test_context::test_context;
use topk_rs::query::{field, filter, select};

mod utils;
use utils::{dataset, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_union_eq(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("user_ratings").eq(10u64)).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_union_starts_with(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("user_ratings", field("user_ratings")),
            ])
            .filter(field("user_ratings").starts_with("good"))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_union_contains(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    for filter_expr in vec![
        field("user_ratings").contains(3u64),
        field("user_ratings").contains(3i64),
        field("user_ratings").contains(3.0f32),
    ] {
        let results = ctx
            .client
            .collection(&collection.name)
            .query(
                select([("user_ratings", field("user_ratings"))])
                    .filter(filter_expr)
                    .topk(field("published_year"), 100, true),
                None,
                None,
            )
            .await
            .expect("could not query");

        assert_doc_ids!(results, ["catcher", "hobbit"]);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_union_contains_both_string_and_list(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("user_ratings", field("user_ratings")),
            ])
            .filter(field("user_ratings").contains("good"))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["gatsby", "lotr", "pride"]);
}
