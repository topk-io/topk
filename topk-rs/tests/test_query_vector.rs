use std::collections::HashMap;
use test_context::test_context;
use topk_protos::v1::data::{
    stage::select_stage::SelectExpr, Document, FunctionExpr, LogicalExpr, Query, Stage, Vector,
};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

fn is_sorted(result: &[Document], field: &str) -> bool {
    result
        .iter()
        .map(|d| {
            d.fields
                .get(field)
                .and_then(|v| v.as_f32())
                .expect("missing sorting field")
        })
        .is_sorted()
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([
                    (
                        "title".to_string(),
                        SelectExpr::logical(LogicalExpr::field("title".to_string())),
                    ),
                    (
                        "summary_distance".to_string(),
                        SelectExpr::function(FunctionExpr::vector_distance(
                            "summary_embedding".to_string(),
                            Vector::float(vec![2.0; 16]),
                        )),
                    ),
                ])),
                Stage::topk(LogicalExpr::field("summary_distance"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&result, "summary_distance"));
    assert_fields!(&result, ["_id", "title", "summary_distance"]);
    assert_doc_ids!(result, ["1984", "pride", "mockingbird"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance_nullable(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "summary_distance".to_string(),
                    SelectExpr::function(FunctionExpr::vector_distance(
                        "nullable_embedding".to_string(),
                        Vector::float(vec![3.0; 16]),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("summary_distance"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&result, "summary_distance"));
    assert_doc_ids!(result, ["1984", "mockingbird", "catcher"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance_u8_vector(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "summary_distance".to_string(),
                    SelectExpr::function(FunctionExpr::vector_distance(
                        "scalar_embedding".to_string(),
                        Vector::byte(vec![8; 16]),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("summary_distance"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&result, "summary_distance"));
    assert_doc_ids!(result, ["harry", "1984", "catcher"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance_binary_vector(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "summary_distance".to_string(),
                    SelectExpr::function(FunctionExpr::vector_distance(
                        "binary_embedding".to_string(),
                        Vector::byte(vec![0, 1]),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("summary_distance"), 2, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&result, "summary_distance"));
    assert_doc_ids!(result, ["1984", "mockingbird"]);
}
