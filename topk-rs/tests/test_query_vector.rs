use test_context::test_context;
use topk_rs::data::{f32_vector, u8_vector};
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
async fn test_query_vector_distance_nullable(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("nullable_embedding", f32_vector(vec![3.0; 16])),
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
                fns::vector_distance("scalar_embedding", u8_vector(vec![8; 16])),
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
async fn test_query_vector_distance_binary_vector(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("binary_embedding", u8_vector(vec![0, 1])),
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
