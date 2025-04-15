use std::collections::HashMap;
use test_context::test_context;
use topk_protos::{
    doc, schema,
    v1::{
        control::FieldSpec,
        data::{
            stage::{filter_stage::FilterExpr, select_stage::SelectExpr},
            text_expr::Term,
            FunctionExpr, LogicalExpr, Query, Stage, TextExpr,
        },
    },
};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_schema(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    for (field, _) in collection.schema.iter() {
        assert!(
            !field.starts_with("_"),
            "Schema contains reserved field: {}",
            field
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_create_with_invalid_model(ctx: &mut ProjectTestContext) {
    let schema = schema!(
        "title" => FieldSpec::semantic(true, Some("definitely-does-not-exist".into()), None),
    );

    let err = ctx
        .client
        .collections()
        .create(ctx.wrap("semantic"), schema)
        .await
        .expect_err("should not create collection");

    assert!(matches!(err, Error::SchemaValidationError(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_write_docs(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(Query::new(vec![Stage::count()]), None, None)
        .await
        .expect("could not query");

    assert_eq!(result, vec![doc!("_count" => 10_u64)]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "sim".to_string(),
                    SelectExpr::function(FunctionExpr::semantic_similarity(
                        "title".to_string(),
                        "dummy".to_string(),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("sim"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 3);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_with_text_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "sim".to_string(),
                    SelectExpr::function(FunctionExpr::semantic_similarity(
                        "title".to_string(),
                        "dummy".to_string(),
                    )),
                )])),
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    true,
                    vec![Term {
                        token: "love".to_string(),
                        field: Some("summary".to_string()),
                        weight: 1.0,
                    }],
                ))),
                Stage::topk(LogicalExpr::field("sim"), 3, true),
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
async fn test_semantic_index_query_with_missing_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "sim".to_string(),
                    SelectExpr::function(FunctionExpr::semantic_similarity(
                        "published_year".to_string(),
                        "dummy".to_string(),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("sim"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect_err("should not query");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_multiple_fields(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([
                    (
                        "title_sim".to_string(),
                        SelectExpr::function(FunctionExpr::semantic_similarity(
                            "title".to_string(),
                            "dummy".to_string(),
                        )),
                    ),
                    (
                        "summary_sim".to_string(),
                        SelectExpr::function(FunctionExpr::semantic_similarity(
                            "summary".to_string(),
                            "query".to_string(),
                        )),
                    ),
                ])),
                Stage::topk(
                    LogicalExpr::add(
                        LogicalExpr::field("title_sim"),
                        LogicalExpr::field("summary_sim"),
                    ),
                    5,
                    true,
                ),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 5);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_and_rerank_with_missing_model(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "sim".to_string(),
                    SelectExpr::function(FunctionExpr::semantic_similarity(
                        "title".to_string(),
                        "dummy".to_string(),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("sim"), 3, true),
                Stage::rerank(Some("definitely-does-not-exist".into()), None, vec![], None),
            ]),
            None,
            None,
        )
        .await
        .expect_err("should not query");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_and_rerank(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "sim".to_string(),
                    SelectExpr::function(FunctionExpr::semantic_similarity(
                        "title".to_string(),
                        "dummy".to_string(),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("sim"), 3, true),
                Stage::rerank(Some("dummy".into()), None, vec![], None),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 3);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_and_rerank_multiple_semantic_sim_explicit(
    ctx: &mut ProjectTestContext,
) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([
                    (
                        "title_sim".to_string(),
                        SelectExpr::function(FunctionExpr::semantic_similarity(
                            "title".to_string(),
                            "dummy".to_string(),
                        )),
                    ),
                    (
                        "summary_sim".to_string(),
                        SelectExpr::function(FunctionExpr::semantic_similarity(
                            "summary".to_string(),
                            "query".to_string(),
                        )),
                    ),
                ])),
                Stage::topk(
                    LogicalExpr::add(
                        LogicalExpr::field("title_sim"),
                        LogicalExpr::field("summary_sim"),
                    ),
                    5,
                    true,
                ),
                Stage::rerank(
                    Some("dummy".into()),
                    Some("query string".into()),
                    vec!["title".to_string(), "summary".to_string()],
                    None,
                ),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 5);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_and_rerank_multiple_semantic_sim_implicit(
    ctx: &mut ProjectTestContext,
) {
    let collection = dataset::semantic::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([
                    (
                        "title_sim".to_string(),
                        SelectExpr::function(FunctionExpr::semantic_similarity(
                            "title".to_string(),
                            "dummy".to_string(),
                        )),
                    ),
                    (
                        "summary_sim".to_string(),
                        SelectExpr::function(FunctionExpr::semantic_similarity(
                            "summary".to_string(),
                            "query".to_string(),
                        )),
                    ),
                ])),
                Stage::topk(
                    LogicalExpr::add(
                        LogicalExpr::field("title_sim"),
                        LogicalExpr::field("summary_sim"),
                    ),
                    5,
                    true,
                ),
                Stage::rerank(Some("dummy".into()), None, vec![], None),
            ]),
            None,
            None,
        )
        .await
        .expect_err("should not query");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
