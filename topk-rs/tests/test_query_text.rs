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
            filter(r#match("love", Some("summary"), None, false)).topk(
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
            filter(r#match("love", Some("summary"), None, false)).topk(
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
            filter(r#match("LOVE", Some("summary"), None, false).or(r#match(
                "ring",
                Some("title"),
                None,
                false,
            )))
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
            filter(r#match("LOVE", Some("summary"), None, false).and(r#match(
                "class",
                Some("summary"),
                None,
                false,
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
            filter(r#match("the", Some("summary"), None, false)).topk(
                field("published_year"),
                100,
                true,
            ),
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

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_matches_single_term(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    for match_expr in [
        filter(field("summary").match_any("love")),
        filter(field("summary").match_all("love")),
    ] {
        let result = ctx
            .client
            .collection(&collection.name)
            .query(
                match_expr.topk(field("published_year"), 100, true),
                None,
                None,
            )
            .await
            .expect("could not query");

        assert_doc_ids!(result, ["pride", "gatsby"]);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_match_all_two_terms(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("summary").match_all("love class")).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_match_any_two_terms(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("summary").match_any("love ring")).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "gatsby", "lotr"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_matches_with_logical_expr(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("summary").match_all("love class") | field("published_year").eq(1925))
                .topk(field("published_year"), 10, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_text_matches_on_invalid_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("published_year").match_all("love class")).count(),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
