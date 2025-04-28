use test_context::test_context;
mod utils;
use topk_rs::{
    query::{field, filter, fns, r#match, select},
    Error,
};
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_filter_single_term_disjunctive(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(r#match("love", Some("summary"), None)).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_filter_single_term_conjunctive(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(r#match("love", Some("summary"), None)).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["gatsby", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_filter_two_terms_disjunctive(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(r#match("LOVE", Some("summary"), None).or(r#match("ring", Some("title"), None)))
                .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "gatsby", "lotr"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_filter_two_terms_conjunctive(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(r#match("LOVE", Some("summary"), None).and(r#match(
                "class",
                Some("summary"),
                None,
            )))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_filter_stop_word(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(r#match("the", Some("summary"), None)).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, Vec::<String>::new());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_bm25_without_text_queries(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("bm25_score", fns::bm25_score())])
                .filter(field("_id").eq("pride"))
                .topk(field("bm25_score"), 100, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
