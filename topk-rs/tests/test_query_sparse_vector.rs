use test_context::test_context;
use topk_rs::proto::v1::data::{SparseVector, Value};
use topk_rs::query::{field, fns, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sparse_vector_distance_f32(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "sparse_f32_distance",
                    fns::vector_distance(
                        "sparse_f32_embedding",
                        SparseVector::f32(
                            vec![0, 1, 2, 3, 4, 5],
                            vec![1.0, 2.0, 3.0, 1.0, 3.0, 2.0],
                        ),
                    ),
                )])
                .topk(field("sparse_f32_distance"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids_ordered!(result, ["1984", "mockingbird", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sparse_vector_distance_u8(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "sparse_u8_distance",
                    fns::vector_distance(
                        "sparse_u8_embedding",
                        SparseVector::u8(vec![0, 1, 2, 3, 4, 5], vec![1, 2, 3, 1, 3, 2]),
                    ),
                )])
                .topk(field("sparse_u8_distance"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids_ordered!(result, ["1984", "mockingbird", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sparse_vector_distance_nullable(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "sparse_u8_distance",
                    fns::vector_distance(
                        "sparse_u8_embedding",
                        SparseVector::u8(vec![0, 1, 2, 3, 4], vec![1, 2, 3, 1, 3]),
                    ),
                )])
                .topk(field("sparse_u8_distance"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_doc_ids_ordered!(result, ["1984", "mockingbird", "pride"]);

    let mut mockingbird = ctx
        .client
        .collection(&collection.name)
        .get(["mockingbird"], None, None, None)
        .await
        .expect("could not get mockingbird")
        .get("mockingbird")
        .cloned()
        .expect("could not get mockingbird");

    mockingbird.insert("sparse_u8_embedding".into(), Value::null());

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![topk_rs::proto::v1::data::Document::from(mockingbird)])
        .await
        .expect("could not upsert mockingbird");

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "sparse_u8_distance",
                    fns::vector_distance(
                        "sparse_u8_embedding",
                        SparseVector::u8(vec![0, 1, 2, 3, 4], vec![1, 2, 3, 1, 3]),
                    ),
                )])
                .topk(field("sparse_u8_distance"), 3, false),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query");
    assert_doc_ids_ordered!(result, ["1984", "pride", "gatsby"]);
}
