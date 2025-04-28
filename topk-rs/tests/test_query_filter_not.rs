use test_context::test_context;
use topk_rs::query::field;
use topk_rs::query::filter;
use topk_rs::query::not;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_not(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(not(field("_id").contains("gatsby"))).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(
        result,
        [
            "harry",
            "lotr",
            "1984",
            "mockingbird",
            "moby",
            "alchemist",
            "catcher",
            "hobbit",
            "pride"
        ]
    );
}
