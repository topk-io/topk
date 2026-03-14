use test_context::test_context;

use topk_rs::error::Error;
use topk_rs::query::{field, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_fetch(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .filter(field("title").eq("1984"))
                .topk(field("published_year"), 100, true)
                .fetch(["summary"]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 1);
    assert_fields!(&results, ["_id", "title", "summary"]);
    assert_eq!(
        results[0].fields.get("summary").unwrap().as_string().unwrap(),
        "A totalitarian regime uses surveillance and mind control to oppress its citizens."
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_fetch_streaming(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .filter(field("title").eq("1984"))
                .limit(100)
                .fetch(["summary"]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 1);
    assert_fields!(&results, ["_id", "title", "summary"]);
    assert_eq!(
        results[0].fields.get("summary").unwrap().as_string().unwrap(),
        "A totalitarian regime uses surveillance and mind control to oppress its citizens."
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_fetch_rejects_select_overlap(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .topk(field("published_year"), 10, true)
                .fetch(["title"]),
            None,
            None,
        )
        .await
        .expect_err("should fail with overlapping select/fetch fields");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
