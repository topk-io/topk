use test_context::test_context;

use topk_rs::error::Error;
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::query::{field, filter, select};

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
                .sort(field("published_year"), true)
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
        results[0]
            .fields
            .get("summary")
            .unwrap()
            .as_string()
            .unwrap(),
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
        results[0]
            .fields
            .get("summary")
            .unwrap()
            .as_string()
            .unwrap(),
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
                .sort(field("published_year"), true)
                .limit(10)
                .fetch(["title"]),
            None,
            None,
        )
        .await
        .expect_err("should fail with overlapping select/fetch fields");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_fetch_wildcard(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("title").eq("1984")).limit(100).fetch(["*"]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 1);
    // Wildcard returns *every* stored field, not just the ones named in the query — the
    // opposite of the named-fetch tests above, which only ever see what they asked for.
    let fields: std::collections::HashSet<&str> =
        results[0].fields.keys().map(|k| k.as_str()).collect();
    for expected in ["_id", "title", "published_year", "summary"] {
        assert!(fields.contains(expected), "missing {expected}: {fields:?}");
    }
    assert!(
        fields.len() > 4,
        "wildcard should return more than just the fields named in the query: {fields:?}"
    );
    assert_eq!(
        results[0]
            .fields
            .get("summary")
            .unwrap()
            .as_string()
            .unwrap(),
        "A totalitarian regime uses surveillance and mind control to oppress its citizens."
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_fetch_wildcard_select_wins_on_overlap(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            // Filter on the real `title` *before* select() overwrites it — otherwise every
            // row's title is already "OVERRIDDEN" by the time the filter runs.
            filter(field("title").eq("1984"))
                .select([("title", LogicalExpr::literal("OVERRIDDEN"))])
                .limit(100)
                .fetch(["*"]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results.len(), 1);
    // `title` was explicitly computed by `select()`; the wildcard fetch's raw `title`
    // must not clobber it, but `summary`/`published_year` still come through from fetch.
    assert_eq!(
        results[0].fields.get("title").unwrap().as_string().unwrap(),
        "OVERRIDDEN"
    );
    assert_eq!(
        results[0]
            .fields
            .get("summary")
            .unwrap()
            .as_string()
            .unwrap(),
        "A totalitarian regime uses surveillance and mind control to oppress its citizens."
    );
}
