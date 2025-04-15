use std::collections::{HashMap, HashSet};
use test_context::test_context;
use topk_protos::doc;
use topk_protos::v1::data::{
    stage::{filter_stage::FilterExpr, select_stage::SelectExpr},
    text_expr, FunctionExpr, Query, Stage, TextExpr,
};
use topk_protos::v1::data::{LogicalExpr, Value, Vector};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "literal".to_string(),
                    SelectExpr::logical(LogicalExpr::literal((1.0 as f32).into())),
                )])),
                Stage::filter(FilterExpr::logical(LogicalExpr::eq(
                    LogicalExpr::field("title"),
                    LogicalExpr::literal("1984".into()),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![doc!("_id" => "1984", "literal" => 1.0 as f32)]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_non_existing_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "literal".to_string(),
                    SelectExpr::logical(LogicalExpr::field("non_existing_field")),
                )])),
                Stage::filter(FilterExpr::logical(LogicalExpr::eq(
                    LogicalExpr::field("title"),
                    LogicalExpr::literal("1984".into()),
                ))),
                Stage::topk(LogicalExpr::field("published_year"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results, vec![doc!("_id" => "1984")]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_limit(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![Stage::topk(
                LogicalExpr::field("published_year"),
                3,
                true,
            )]),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 3);

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![Stage::topk(
                LogicalExpr::field("published_year"),
                2,
                true,
            )]),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 2);

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![Stage::topk(
                LogicalExpr::field("published_year"),
                1,
                true,
            )]),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 1);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_asc(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "published_year".to_string(),
                    SelectExpr::logical(LogicalExpr::field("published_year")),
                )])),
                Stage::topk(LogicalExpr::field("published_year"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "pride", "published_year" => 1813 as u32),
            doc!("_id" => "moby", "published_year" => 1851 as u32),
            doc!("_id" => "gatsby", "published_year" => 1925 as u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_desc(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "published_year".to_string(),
                    SelectExpr::logical(LogicalExpr::field("published_year")),
                )])),
                Stage::topk(LogicalExpr::field("published_year"), 3, false),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "harry", "published_year" => 1997 as u32),
            doc!("_id" => "alchemist", "published_year" => 1988 as u32),
            doc!("_id" => "mockingbird", "published_year" => 1960 as u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_bm25_score(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "bm25_score".to_string(),
                    SelectExpr::function(FunctionExpr::bm25_score()),
                )])),
                Stage::filter(FilterExpr::text(TextExpr::terms(
                    true,
                    vec![text_expr::Term {
                        token: "pride".to_string(),
                        field: None,
                        weight: 1.0,
                    }],
                ))),
                Stage::topk(LogicalExpr::field("bm25_score"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![doc!("_id" => "pride", "bm25_score" => 2.0774152 as f32)]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_vector_distance(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([(
                    "summary_distance".to_string(),
                    SelectExpr::function(FunctionExpr::vector_distance(
                        "summary_embedding".to_string(),
                        Vector::float(vec![2.0; 16]),
                    )),
                )])),
                Stage::topk(LogicalExpr::field("summary_distance"), 3, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    // note: purposefully not matching on `summary_distance` since the exact value changes over time as we develop the system
    assert_eq!(results.len(), 3);
    assert_eq!(
        results
            .into_iter()
            .map(|d| d.id().unwrap().to_string())
            .collect::<HashSet<_>>(),
        ["1984".into(), "mockingbird".into(), "pride".into()].into()
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_null_field(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    ctx.client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "1984", "a" => Value::null()),
            doc!("_id" => "pride"),
        ])
        .await
        .expect("could not upsert documents");

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            Query::new(vec![
                Stage::select(HashMap::from([
                    (
                        "a".to_string(),
                        SelectExpr::logical(LogicalExpr::field("a")),
                    ),
                    (
                        "b".to_string(),
                        SelectExpr::logical(LogicalExpr::literal(1.into())),
                    ),
                ])),
                Stage::topk(LogicalExpr::field("b"), 100, true),
            ]),
            None,
            None,
        )
        .await
        .expect("could not query");

    // Assert that `a` is null for all documents, even when not specified when upserting
    assert_eq!(
        results
            .into_iter()
            .map(|d| d.fields.get("a").unwrap().clone())
            .collect::<Vec<_>>(),
        vec![Value::null(), Value::null()]
    );
}
