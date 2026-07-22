use futures::TryStreamExt;
use test_context::test_context;

use topk_rs::data::literal;
use topk_rs::query::{field, filter};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_matched_count_reported_for_sorted(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let stream = ctx
        .client
        .collection(&collection.name)
        .query_stream(
            filter(field("published_year").gte(literal(1950_u32)))
                .sort("published_year")
                .limit(2),
            None,
            None,
        )
        .await
        .expect("could not query");

    // 5 books from 1950 on; matched is the filter count, not hits or corpus.
    assert_eq!(stream.matched_count(), Some(5));

    let docs = stream
        .try_collect::<Vec<_>>()
        .await
        .expect("could not collect");
    assert_eq!(docs.len(), 2);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_matched_count_absent_for_unsorted(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let stream = ctx
        .client
        .collection(&collection.name)
        .query_stream(
            filter(field("published_year").gte(literal(1950_u32))).limit(2),
            None,
            None,
        )
        .await
        .expect("could not query");

    // A bare limit exits early, so no complete count exists to report.
    assert_eq!(stream.matched_count(), None);
}
