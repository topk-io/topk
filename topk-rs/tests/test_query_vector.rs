use test_context::test_context;
use topk_rs::query::{field, fns, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

use crate::utils::is_sorted;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "summary_distance",
                    fns::vector_distance("summary_embedding", vec![2.0; 16]),
                )])
                .topk(field("summary_distance"), 3, true),
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
async fn test_query_vector_distance_without_refine(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let raw = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "summary_distance",
                    fns::vector_distance("summary_embedding", vec![2.0; 16]).skip_refine(true),
                )])
                .topk(field("summary_distance"), 3, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&raw, "summary_distance"));
    assert_doc_ids!(&raw, ["1984", "pride", "mockingbird"]);

    let refined = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "summary_distance",
                    fns::vector_distance("summary_embedding", vec![2.34; 16]),
                )])
                .topk(field("summary_distance"), 3, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&refined, "summary_distance"));
    assert_doc_ids!(&refined, ["1984", "pride", "mockingbird"]);

    // The refined result should be different from the raw result.
    assert_ne!(raw, refined);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance_nullable(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("nullable_embedding", vec![3.0f32; 16]),
            )])
            .topk(field("summary_distance"), 3, true),
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
            select([(
                "summary_distance",
                fns::vector_distance("scalar_embedding", vec![8u8; 16]),
            )])
            .topk(field("summary_distance"), 3, true),
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
async fn test_query_vector_distance_i8_vector(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("scalar_i8_embedding", vec![-10i8; 16]),
            )])
            .topk(field("summary_distance"), 3, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&result, "summary_distance"));
    assert_doc_ids!(result, ["pride", "1984", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_vector_distance_binary_vector(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("binary_embedding", vec![0u8, 1]),
            )])
            .topk(field("summary_distance"), 2, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(is_sorted(&result, "summary_distance"));
    assert_doc_ids!(result, ["1984", "mockingbird"]);
}
