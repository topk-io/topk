use std::collections::HashMap;

use test_context::test_context;
use topk_rs::proto::v1::data::{AggregateExpr, LogicalExpr};
use topk_rs::query::{field, filter, group_by};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

// published_year per book in the `books` dataset:
//   mockingbird 1960, 1984 1949, pride 1813, gatsby 1925, catcher 1951,
//   moby 1851, hobbit 1937, harry 1997, lotr 1954, alchemist 1988
//
// `published_year < 1940` splits them into:
//   old  (4): pride 1813, gatsby 1925, moby 1851, hobbit 1937
//   new  (6): mockingbird 1960, 1984 1949, catcher 1951, harry 1997, lotr 1954, alchemist 1988

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_bool_key_expr(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("count", AggregateExpr::count(None))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 2);

    for row in result {
        if row.fields["is_old"].as_bool().unwrap() {
            assert_eq!(row.fields["count"].as_u64().unwrap(), 4);
        } else {
            assert_eq!(row.fields["count"].as_u64().unwrap(), 6);
        }
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_count(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("count", AggregateExpr::count(None))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let counts: HashMap<bool, u64> = result
        .iter()
        .map(|row| {
            (
                row.fields["is_old"].as_bool().unwrap(),
                row.fields["count"].as_u64().unwrap(),
            )
        })
        .collect();

    assert_eq!(counts, HashMap::from([(true, 4), (false, 6)]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_count_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // `nullable_importance` is only present on `mockingbird` (new, 2.0) and `moby` (old, 5.0).
    // `count(None)` counts every row, while `count(Some(field))` counts non-null values.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [
                    ("total", AggregateExpr::count(None)),
                    (
                        "with_importance",
                        AggregateExpr::count(Some("nullable_importance".to_string())),
                    ),
                ],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let by_group: HashMap<bool, (u64, u64)> = result
        .iter()
        .map(|row| {
            (
                row.fields["is_old"].as_bool().unwrap(),
                (
                    row.fields["total"].as_u64().unwrap(),
                    row.fields["with_importance"].as_u64().unwrap(),
                ),
            )
        })
        .collect();

    assert_eq!(by_group, HashMap::from([(true, (4, 1)), (false, (6, 1))]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_sum(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("total_year", AggregateExpr::sum("published_year"))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let sums: HashMap<bool, u32> = result
        .iter()
        .map(|row| {
            (
                row.fields["is_old"].as_bool().unwrap(),
                row.fields["total_year"].as_u32().unwrap(),
            )
        })
        .collect();

    // old: 1813 + 1925 + 1851 + 1937 = 7526
    // new: 1960 + 1949 + 1951 + 1997 + 1954 + 1988 = 11799
    assert_eq!(sums, HashMap::from([(true, 7526), (false, 11799)]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_min_max(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [
                    ("oldest", AggregateExpr::min("published_year")),
                    ("newest", AggregateExpr::max("published_year")),
                ],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let by_group: HashMap<bool, (u32, u32)> = result
        .iter()
        .map(|row| {
            (
                row.fields["is_old"].as_bool().unwrap(),
                (
                    row.fields["oldest"].as_u32().unwrap(),
                    row.fields["newest"].as_u32().unwrap(),
                ),
            )
        })
        .collect();

    // old: min 1813 (pride), max 1937 (hobbit)
    // new: min 1949 (1984), max 1997 (harry)
    assert_eq!(
        by_group,
        HashMap::from([(true, (1813, 1937)), (false, (1949, 1997))])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_avg(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("avg_year", AggregateExpr::avg("published_year"))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let avgs: HashMap<bool, f64> = result
        .iter()
        .map(|row| {
            (
                row.fields["is_old"].as_bool().unwrap(),
                row.fields["avg_year"].as_f64().unwrap(),
            )
        })
        .collect();

    // old: 7526 / 4 = 1881.5, new: 11799 / 6 = 1966.5
    assert!((avgs[&true] - 1881.5).abs() < 1e-9);
    assert!((avgs[&false] - 1966.5).abs() < 1e-9);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_multiple_aggregations(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [
                    ("count", AggregateExpr::count(None)),
                    ("total_year", AggregateExpr::sum("published_year")),
                    ("oldest", AggregateExpr::min("published_year")),
                    ("newest", AggregateExpr::max("published_year")),
                    ("avg_year", AggregateExpr::avg("published_year")),
                ],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 2);

    for row in result {
        if row.fields["is_old"].as_bool().unwrap() {
            assert_eq!(row.fields["count"].as_u64().unwrap(), 4);
            assert_eq!(row.fields["total_year"].as_u32().unwrap(), 7526);
            assert_eq!(row.fields["oldest"].as_u32().unwrap(), 1813);
            assert_eq!(row.fields["newest"].as_u32().unwrap(), 1937);
            assert!((row.fields["avg_year"].as_f64().unwrap() - 1881.5).abs() < 1e-9);
        } else {
            assert_eq!(row.fields["count"].as_u64().unwrap(), 6);
            assert_eq!(row.fields["total_year"].as_u32().unwrap(), 11799);
            assert_eq!(row.fields["oldest"].as_u32().unwrap(), 1949);
            assert_eq!(row.fields["newest"].as_u32().unwrap(), 1997);
            assert!((row.fields["avg_year"].as_f64().unwrap() - 1966.5).abs() < 1e-9);
        }
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_multiple_keys(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Two independent key expressions:
    //   is_old  = published_year < 1940
    //   is_19th = published_year < 1900
    //
    //   pride 1813:  (old, 19th)
    //   moby  1851:  (old, 19th)
    //   gatsby 1925: (old, !19th)
    //   hobbit 1937: (old, !19th)
    //   the other 6: (!old, !19th)
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [
                    ("is_old", field("published_year").lt(1940 as u32)),
                    ("is_19th", field("published_year").lt(1900 as u32)),
                ],
                [("count", AggregateExpr::count(None))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let counts: HashMap<(bool, bool), u64> = result
        .iter()
        .map(|row| {
            (
                (
                    row.fields["is_old"].as_bool().unwrap(),
                    row.fields["is_19th"].as_bool().unwrap(),
                ),
                row.fields["count"].as_u64().unwrap(),
            )
        })
        .collect();

    assert_eq!(
        counts,
        HashMap::from([((true, true), 2), ((true, false), 2), ((false, false), 6),])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_with_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Filter to books published in 1940 or later (drops the 4 "old" books),
    // then group the remaining 6 by whether they were published after 1980.
    //   after 1980: harry 1997, alchemist 1988          -> 2
    //   otherwise:  mockingbird, 1984, catcher, lotr     -> 4
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("published_year").gte(1940 as u32)).group_by(
                [("recent", field("published_year").gt(1980 as u32))],
                [("count", AggregateExpr::count(None))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let counts: HashMap<bool, u64> = result
        .iter()
        .map(|row| {
            (
                row.fields["recent"].as_bool().unwrap(),
                row.fields["count"].as_u64().unwrap(),
            )
        })
        .collect();

    assert_eq!(counts, HashMap::from([(true, 2), (false, 4)]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_with_projected_columns(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // A preceding `select` projects computed columns (`year`, `old`) which the
    // group_by stage then references in both its key and its aggregations.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            topk_rs::query::select([
                ("year", field("published_year")),
                ("old", field("published_year").lt(1940 as u32)),
            ])
            .group_by(
                [("old", field("old"))],
                [
                    ("count", AggregateExpr::count(None)),
                    ("total_year", AggregateExpr::sum("year")),
                ],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    let by_group: HashMap<bool, (u64, u32)> = result
        .iter()
        .map(|row| {
            (
                row.fields["old"].as_bool().unwrap(),
                (
                    row.fields["count"].as_u64().unwrap(),
                    row.fields["total_year"].as_u32().unwrap(),
                ),
            )
        })
        .collect();

    assert_eq!(
        by_group,
        HashMap::from([(true, (4, 7526)), (false, (6, 11799))])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_then_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Group into old (4) / new (6), then keep only groups with more than 4 members.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("count", AggregateExpr::count(None))],
            )
            .filter(field("count").gt(4 as u64)),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].fields["is_old"].as_bool().unwrap(), false);
    assert_eq!(result[0].fields["count"].as_u64().unwrap(), 6);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_then_sort_limit(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Group, then take the single largest group by count.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("count", AggregateExpr::count(None))],
            )
            .sort(field("count"), false)
            .limit(1),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].fields["is_old"].as_bool().unwrap(), false);
    assert_eq!(result[0].fields["count"].as_u64().unwrap(), 6);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_then_select(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // A `select` after group_by projects a subset / renaming of the grouped output.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("count", AggregateExpr::count(None))],
            )
            .select([("n", field("count"))]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 2);

    let ns: Vec<u64> = {
        let mut ns = result
            .iter()
            .map(|row| row.fields["n"].as_u64().unwrap())
            .collect::<Vec<_>>();
        ns.sort();
        ns
    };
    assert_eq!(ns, vec![4, 6]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_empty_keys(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                Vec::<(String, LogicalExpr)>::new(),
                [("count", AggregateExpr::count(None))],
            ),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s.contains("at least one key")),
        "unexpected error: {err:?}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_empty_aggregations(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                Vec::<(String, AggregateExpr)>::new(),
            ),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s.contains("at least one aggregation")),
        "unexpected error: {err:?}"
    );
}
