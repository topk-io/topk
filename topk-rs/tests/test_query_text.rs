use std::collections::HashMap;
use test_context::test_context;
use topk_protos::v1::data::{
    stage::{filter_stage::FilterExpr, select_stage::SelectExpr},
    text_expr::Term,
    FunctionExpr, LogicalExpr, Query, Stage, TextExpr,
};

mod utils;
use topk_rs::Error;
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
            Query::new(vec![
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    false,
                    vec![Term {
                        token: "love".to_string(),
                        field: Some("summary".to_string()),
                        weight: 1.0,
                    }],
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
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
            Query::new(vec![
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    true,
                    vec![Term {
                        token: "love".to_string(),
                        field: Some("summary".to_string()),
                        weight: 1.0,
                    }],
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
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
            Query::new(vec![
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    false,
                    vec![
                        Term {
                            token: "LOVE".to_string(),
                            field: Some("summary".to_string()),
                            weight: 1.0,
                        },
                        Term {
                            token: "ring".to_string(),
                            field: Some("title".to_string()),
                            weight: 1.0,
                        },
                    ],
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
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
            Query::new(vec![
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    true,
                    vec![
                        Term {
                            token: "LOVE".to_string(),
                            field: Some("summary".to_string()),
                            weight: 1.0,
                        },
                        Term {
                            token: "class".to_string(),
                            field: Some("summary".to_string()),
                            weight: 1.0,
                        },
                    ],
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
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
            Query::new(vec![
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    true,
                    vec![Term {
                        token: "the".to_string(),
                        field: None,
                        weight: 1.0,
                    }],
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
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
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "bm25_score".to_string(),
                    SelectExpr::function(FunctionExpr::bm25_score()),
                )])),
                Stage::filter(FilterExpr::logical(LogicalExpr::eq(
                    LogicalExpr::field("_id"),
                    LogicalExpr::literal("pride".into()),
                ))),
                Stage::topk(LogicalExpr::field("bm25_score"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect_err("should have failed");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
