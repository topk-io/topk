use chrono::{TimeZone, Utc};
use test_context::test_context;

use topk_rs::proto::v1::data::logical_expr::Interval;
use topk_rs::query::now;
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

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_date_trunc_group_by(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))])
                .group_by(
                    [("year", field("published_ts").date_trunc("year", "UTC"))],
                    [("count", AggregateExpr::count(None))],
                )
                .sort([(field("year"), SortOrder::Asc)])
                .limit(20),
            None,
            None,
        )
        .await
        .expect("could not query");

    for doc in &result {
        let at = doc
            .fields
            .get("year")
            .and_then(|v| v.as_datetime())
            .unwrap();
        assert_eq!(
            at.to_rfc3339(),
            format!("{}-01-01T00:00:00+00:00", at.format("%Y"))
        );
    }
    assert!(!result.is_empty());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_date_trunc_follows_time_zone(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let day = |zone: &str| {
        let zone = zone.to_string();
        async {
            ctx.client
                .collection(&collection.name)
                .query(
                    select([("day", field("published_ts").date_trunc("day", zone))])
                        .sort([(field("day"), SortOrder::Asc)])
                        .limit(1),
                    None,
                    None,
                )
                .await
                .expect("could not query")
        }
    };

    let utc = day("UTC").await;
    let auckland = day("Pacific/Auckland").await;

    assert_ne!(
        utc[0].fields.get("day").and_then(|v| v.as_timestamp()),
        auckland[0].fields.get("day").and_then(|v| v.as_timestamp()),
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_date_add_is_calendar_aware(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "plus_month",
                    field("published_ts").date_add(
                        Interval {
                            months: 1,
                            ..Default::default()
                        },
                        "UTC",
                    ),
                ),
                (
                    "plus_30d",
                    field("published_ts")
                        .date_add(std::time::Duration::from_secs(30 * 86_400), "UTC"),
                ),
            ])
            .sort([(field("plus_month"), SortOrder::Asc)])
            .limit(20),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(result.iter().any(|doc| {
        let m = doc.fields.get("plus_month").and_then(|v| v.as_timestamp());
        let d = doc.fields.get("plus_30d").and_then(|v| v.as_timestamp());
        m != d
    }));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_now_is_one_instant_per_query(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("a", now()),
                ("b", now()),
                ("day", now().date_trunc("day", "UTC")),
            ])
            .limit(1),
            None,
            None,
        )
        .await
        .expect("could not query");

    let a = result[0]
        .fields
        .get("a")
        .and_then(|v| v.as_timestamp())
        .unwrap();
    let b = result[0]
        .fields
        .get("b")
        .and_then(|v| v.as_timestamp())
        .unwrap();
    let day = result[0]
        .fields
        .get("day")
        .and_then(|v| v.as_timestamp())
        .unwrap();

    assert_eq!(a, b);
    assert_eq!(day, a - a.rem_euclid(86_400_000));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_now_is_substituted_in_every_stage(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // `now` resolves during expr conversion, so a filter, a grouping key and a sort all
    // see the same instant.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("published_ts").lt(now()))
                .group_by(
                    [("year", now().date_trunc("year", "UTC"))],
                    [("count", AggregateExpr::count(None))],
                )
                .sort([(now().date_trunc("day", "UTC"), SortOrder::Asc)])
                .limit(10),
            None,
            None,
        )
        .await
        .expect("could not query");

    // Every book predates the query, and they all fall in the same `now`-derived bucket.
    assert_eq!(result.len(), 1);
    assert!(
        result[0]
            .fields
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap()
            > 0
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_interval_components_apply_independently(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Months, days and millis apply independently, so a month is not 30 days and a
    // calendar day is not always 24 hours.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "combined",
                    field("published_ts").date_add(
                        Interval {
                            months: 1,
                            days: 15,
                            ..Default::default()
                        },
                        "Europe/Prague",
                    ),
                ),
                (
                    "stepwise",
                    field("published_ts")
                        .date_add(
                            Interval {
                                months: 1,
                                ..Default::default()
                            },
                            "Europe/Prague",
                        )
                        .date_add(
                            Interval {
                                days: 15,
                                ..Default::default()
                            },
                            "Europe/Prague",
                        ),
                ),
            ])
            .sort([(field("combined"), SortOrder::Asc)])
            .limit(20),
            None,
            None,
        )
        .await
        .expect("could not query");

    // One interval applied at once must equal the same components applied in order.
    for doc in &result {
        assert_eq!(
            doc.fields.get("combined").and_then(|v| v.as_timestamp()),
            doc.fields.get("stepwise").and_then(|v| v.as_timestamp()),
        );
    }
    assert!(!result.is_empty());
}
