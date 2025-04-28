use test_context::test_context;
use topk_protos::v1::data::Document;
use topk_rs::data::Vector;
use topk_rs::query::{field, fns, select};

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
                fns::vector_distance("nullable_embedding", Vector::F32(vec![3.0; 16])),
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
                fns::vector_distance("scalar_embedding", Vector::U8(vec![8; 16])),
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
                fns::vector_distance("binary_embedding", Vector::U8(vec![0, 1])),
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
