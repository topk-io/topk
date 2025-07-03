use test_context::test_context;
use topk_rs::data::f32_vector;
use topk_rs::query::{field, filter, fns, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

use topk_rs::Error;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_clamping(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "summary_distance",
                    fns::vector_distance("summary_embedding", f32_vector(vec![2.0; 16])),
                ),
                ("bm25_score", fns::bm25_score()),
            ])
            .topk(
                field("bm25_score")
                    .max(3)
                    .min(10)
                    .add(field("summary_distance").mul(0.5)),
                2,
                true,
            ),
            None,
            None,
        )
        .await
        .expect_err("max, min not implemented yet");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_pow_sqrt(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "summary_distance",
                    fns::vector_distance("summary_embedding", f32_vector(vec![2.0; 16])),
                ),
                ("bm25_score", fns::bm25_score()),
            ])
            .topk(
                field("bm25_score")
                    .pow(1.5)
                    .add(field("summary_distance").pow(2))
                    .sqrt(),
                2,
                true,
            ),
            None,
            None,
        )
        .await
        .expect_err("pow, sqrt not implemented yet");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_exp_precision(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(
                field("published_year")
                    .exp()
                    .ln()
                    .sub(1988 as u32)
                    .abs()
                    .lte(10e-6 as u32),
            )
            .topk(field("published_year"), 2, true),
            None,
            None,
        )
        .await
        .expect_err("exp, ln, abs not implemented yet");

    assert!(matches!(err, Error::InvalidArgument(_)));
    // assert_doc_ids!(result, ["alchemist"]);
}
