use chrono::{TimeZone, Utc};
use test_context::test_context;

use topk_rs::{
    data::literal,
    doc,
    proto::v1::data::AggregateExpr,
    query::{field, filter, select, SortOrder},
};

mod utils;
use utils::{dataset, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_filter_timestamp(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(
                field("published_ts")
                    .lt(literal(Utc.with_ymd_and_hms(1929, 1, 1, 0, 0, 0).unwrap())),
            )
            .limit(20),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "moby", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_date_part_eq_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(
                field("published_ts")
                    .date_part("year")
                    .eq(field("published_year")),
            )
            .count(),
            None,
            None,
        )
        .await
        .expect("could not query");

    let count = result[0].fields["_count"].as_u64().unwrap();
    assert_eq!(count, 10);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_date_part_lt_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("published_ts").date_part("month").lt(literal(6))).limit(10),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["gatsby", "pride", "alchemist"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_date_part_group_by(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))])
                .group_by(
                    [("published_month", field("published_ts").date_part("month"))],
                    [("count", AggregateExpr::count(None))],
                )
                .sort([(field("published_month"), SortOrder::Asc)])
                .limit(20),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("published_month" => 1i32, "count" => 2u64),
            doc!("published_month" => 4i32, "count" => 1u64),
            doc!("published_month" => 6i32, "count" => 2u64),
            doc!("published_month" => 7i32, "count" => 3u64),
            doc!("published_month" => 9i32, "count" => 1u64),
            doc!("published_month" => 10i32, "count" => 1u64),
        ],
    );
}
