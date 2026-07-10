use rstest::rstest;
use std::collections::{HashMap, HashSet};
use test_context::AsyncTestContext;
use topk_rs::proto::v1::data::stage::sort_stage::SortOrder;
use topk_rs::proto::v1::data::Query;

use test_context::test_context;
use topk_rs::data::literal;
use topk_rs::proto::v1::data::Document;
use topk_rs::query::{field, fns, r#match, select};

mod utils;
use topk_rs::Error;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_bare_limit(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let docs = dataset::books::docs();

    let result = ctx
        .client
        .collection(&collection.name)
        .query(select([("_id", field("_id"))]).limit(100), None, None)
        .await
        .expect("could not query");

    let expected_ids = docs
        .iter()
        .map(|doc| doc.id().unwrap().to_string())
        .collect::<Vec<_>>();

    assert_eq!(result.len(), 10);
    assert_fields!(&result, ["_id"]);
    assert_doc_ids!(result, expected_ids);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_limit_select_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("summary", field("summary")),
                ("is_recent", field("published_year").gt(literal(1980))),
            ])
            .filter(field("_id").lte(literal("hobbit")))
            .limit(3),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 3);
    assert_fields!(&result, ["_id", "summary", "is_recent"]);

    let expected_ids = HashSet::from(["1984", "alchemist", "catcher", "gatsby", "harry", "hobbit"]);
    for doc in result {
        assert!(expected_ids.contains(&doc.id().unwrap()));
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_limit_with_bm25(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("bm25", fns::bm25_score(None, None))])
                .filter(r#match("quest", None, None, true))
                .limit(10),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 2);
    assert_fields!(&result, ["_id", "bm25"]);
    assert_doc_ids!(result, ["moby", "hobbit"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_limit_vector_distance(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // skip_refine is implicitly true
    let result_limit = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "summary_distance",
                    fns::vector_distance("summary_embedding", vec![2.0; 16]),
                )])
                .limit(100),
            None,
            None,
        )
        .await
        .expect("could not query");

    // explicitly set skip_refine to true
    let result_topk = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "summary_distance",
                    fns::vector_distance("summary_embedding", vec![2.0; 16]).skip_refine(true),
                )])
                .sort((field("summary_distance"), SortOrder::Asc))
                .limit(100),
            None,
            None,
        )
        .await
        .expect("could not query");

    let docs_limit = result_limit
        .iter()
        .map(|doc| (doc.id().unwrap().to_string(), doc))
        .collect::<HashMap<String, &Document>>();

    let docs_topk = result_topk
        .iter()
        .map(|doc| (doc.id().unwrap().to_string(), doc))
        .collect::<HashMap<String, &Document>>();

    assert_fields!(&result_limit, ["_id", "title", "summary_distance"]);
    assert_fields!(&result_topk, ["_id", "title", "summary_distance"]);
    assert_eq!(docs_limit.len(), docs_topk.len());

    // vector distance from limit should be the same as the vector distance from topk with skip_refine true
    assert_eq!(docs_limit, docs_topk);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_limit_offset(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // limit(4) after skipping the first 3 docs of that same ordering. The
    // collector is inflated to limit(7) and the skip is applied post-merge.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id"))]).limit(4).offset(3),
            None,
            None,
        )
        .await
        .expect("could not query");

    // We don't guarantee any ordering of the results, so we just check the length.
    assert_eq!(result.len(), 4);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sort_limit_offset(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Books sorted by ascending publication year, then windowed with
    // offset(3).limit(4): skip the 3 oldest (pride, moby, gatsby), take the
    // next 4. The partitions fetch a partial top-7 and the router merges and
    // skips.
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("_id", field("_id")),
                ("published_year", field("published_year")),
            ])
            .sort((field("published_year"), SortOrder::Asc))
            .limit(4)
            .offset(3),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 4);
    assert_fields!(&result, ["_id", "published_year"]);
    assert_doc_ids_ordered!(&result, ["hobbit", "1984", "catcher", "lotr"]);
}

#[tokio::test]
#[rstest]
#[case(
    select([("title", field("title"))])
        .sort((field("published_year"), SortOrder::Asc)).limit(100)
        .limit(100)
)]
#[case(
    select([("title", field("title"))]).limit(100).count()
)]
#[case(
    select([("title", field("title"))]).sort((field("published_year"), SortOrder::Asc))
)]
#[case(
    select([("title", field("title"))])
        .sort((field("published_year"), SortOrder::Asc))
        .sort((field("published_year"), SortOrder::Desc))
)]
#[case(
    select([("title", field("title"))])
        .sort((field("published_year"), SortOrder::Asc)).limit(100)
        .sort((field("published_year"), SortOrder::Asc))
)]
async fn test_query_invalid_collectors(#[case] query: Query) {
    let mut ctx = ProjectTestContext::setup().await;
    let collection = dataset::books::setup(&mut ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(query, None, None)
        .await
        .expect_err("should have failed");

    // Explicitly teardown the context
    ctx.teardown().await;

    assert!(matches!(err, Error::InvalidArgument(_)));
}
