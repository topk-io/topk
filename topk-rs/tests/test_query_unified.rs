use std::collections::{HashMap, HashSet};
use test_context::test_context;
use topk_protos::v1::data::{
    stage::{filter_stage::FilterExpr, select_stage::SelectExpr},
    text_expr::Term,
    FunctionExpr, LogicalExpr, Query, Stage, TextExpr, Vector,
};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_unified(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([
                    (
                        "summary_distance".to_string(),
                        SelectExpr::function(FunctionExpr::vector_distance(
                            "summary_embedding".to_string(),
                            Vector::float(vec![2.0; 16]),
                        )),
                    ),
                    (
                        "bm25_score".to_string(),
                        SelectExpr::function(FunctionExpr::bm25_score()),
                    ),
                ])),
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    false,
                    vec![
                        Term {
                            token: "love".to_string(),
                            field: None,
                            weight: 30.0,
                        },
                        Term {
                            token: "young".to_string(),
                            field: None,
                            weight: 10.0,
                        },
                    ],
                ))),
                Stage::topk(
                    LogicalExpr::add(
                        LogicalExpr::field("bm25_score"),
                        LogicalExpr::mul(
                            LogicalExpr::field("summary_distance"),
                            LogicalExpr::literal((100 as u32).into()),
                        ),
                    ),
                    2,
                    true,
                ),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(result.len() == 2);
    assert_eq!(
        result
            .into_iter()
            .map(|d| d.id().unwrap().to_string())
            .collect::<HashSet<_>>(),
        ["mockingbird".into(), "pride".into()].into()
    );
}
