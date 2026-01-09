use test_context::test_context;

use topk_rs::data::literal;
use topk_rs::proto::v1::control::field_type_matrix::MatrixValueType;
use topk_rs::proto::v1::data::Matrix;
use topk_rs::query::{field, fns, select};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

const Q1: [f32; 2 * 7] = [
    -0.4449, 1.3496, 0.6855, -0.7714, -0.0942, -0.7982, -0.4429, -0.5834, -0.7113, 1.009, 1.1826,
    0.5344, 0.0189, -0.2313,
];

const Q2: [f32; 3 * 7] = [
    1.5269, -0.2615, -0.1201, -1.495, 0.5497, 0.1703, -0.4399, 1.8301, 0.6419, -1.8175, 1.8999,
    -0.3407, 0.5301, -1.1665, -1.6396, 2.2458, 0.1597, 0.8082, 0.2963, 0.1538, 1.3943,
];

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_float(ctx: &mut ProjectTestContext) {
    for dt in [
        MatrixValueType::F32,
        MatrixValueType::F16,
        MatrixValueType::F8,
    ] {
        println!("dt={dt:?}");
        let collection = dataset::multi_vec::setup(ctx, dt).await;

        for (q, expected_ids) in [
            (Q1.to_vec(), ["doc_7", "doc_8", "doc_6"]),
            (Q2.to_vec(), ["doc_0", "doc_6", "doc_8"]),
        ] {
            let result = ctx
                .client
                .collection(&collection.name)
                .query(
                    select([("title", field("title"))])
                        .select([(
                            "dist",
                            fns::multi_vector_distance(
                                "token_embeddings",
                                dataset::multi_vec::cast(dt, Matrix::new(7, q)),
                                None,
                            ),
                        )])
                        .topk(field("dist"), 3, false),
                    None,
                    None,
                )
                .await
                .expect("could not query");

            assert_eq!(result.len(), 3);
            assert_doc_ids_ordered!(result, expected_ids);
        }
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_int(ctx: &mut ProjectTestContext) {
    for (dt, queries) in [
        (
            MatrixValueType::U8,
            [
                (Q1.to_vec(), ["doc_1", "doc_4", "doc_6"]),
                (Q2.to_vec(), ["doc_1", "doc_2", "doc_4"]),
            ],
        ),
        (
            MatrixValueType::I8,
            [
                (Q1.to_vec(), ["doc_7", "doc_8", "doc_6"]),
                (Q2.to_vec(), ["doc_0", "doc_6", "doc_5"]),
            ],
        ),
    ] {
        println!("dt={dt:?}");
        let collection = dataset::multi_vec::setup(ctx, dt).await;

        for (q, expected_ids) in queries {
            let result = ctx
                .client
                .collection(&collection.name)
                .query(
                    select([("title", field("title"))])
                        .select([(
                            "dist",
                            fns::multi_vector_distance(
                                "token_embeddings",
                                dataset::multi_vec::cast(dt, Matrix::new(7, q)),
                                None,
                            ),
                        )])
                        .topk(field("dist"), 3, false),
                    None,
                    None,
                )
                .await
                .expect("could not query");

            assert_eq!(result.len(), 3);
            assert_doc_ids_ordered!(result, expected_ids);
        }
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_with_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::multi_vec::setup(ctx, MatrixValueType::F32).await;

    for (q, expected_ids) in [
        (Q1.to_vec(), ["doc_7", "doc_6", "doc_1"]),
        (Q2.to_vec(), ["doc_0", "doc_6", "doc_5"]),
    ] {
        let result = ctx
            .client
            .collection(&collection.name)
            .query(
                select([("title", field("title"))])
                    .select([(
                        "dist",
                        fns::multi_vector_distance("token_embeddings", Matrix::new(7, q), None),
                    )])
                    .filter(field("_id").neq(literal("doc_8")))
                    .topk(field("dist"), 3, false),
                None,
                None,
            )
            .await
            .expect("could not query");

        assert_eq!(result.len(), 3);
        assert_doc_ids_ordered!(result, expected_ids);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_with_invalid_dim(ctx: &mut ProjectTestContext) {
    let collection = dataset::multi_vec::setup(ctx, MatrixValueType::F32).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "dist",
                    fns::multi_vector_distance(
                        "token_embeddings",
                        Matrix::new(2, Q1.to_vec()),
                        None,
                    ),
                )])
                .topk(field("dist"), 3, false),
            None,
            None,
        )
        .await
        .expect_err("Query should fail");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_with_invalid_data_type(ctx: &mut ProjectTestContext) {
    let collection = dataset::multi_vec::setup(ctx, MatrixValueType::F32).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "dist",
                    fns::multi_vector_distance(
                        "token_embeddings",
                        dataset::multi_vec::cast(MatrixValueType::F16, Matrix::new(7, Q1.to_vec())),
                        None,
                    ),
                )])
                .topk(field("dist"), 3, false),
            None,
            None,
        )
        .await
        .expect_err("Query should fail");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_with_empty_query(ctx: &mut ProjectTestContext) {
    let collection = dataset::multi_vec::setup(ctx, MatrixValueType::F32).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "dist",
                    fns::multi_vector_distance(
                        "token_embeddings",
                        Matrix::new(7, vec![0.0f32; 0]),
                        None,
                    ),
                )])
                .topk(field("dist"), 3, false),
            None,
            None,
        )
        .await
        .expect_err("Query should fail");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_multi_vector_with_missing_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([(
                    "dist",
                    fns::multi_vector_distance(
                        "token_embeddings",
                        Matrix::new(7, Q1.to_vec()),
                        None,
                    ),
                )])
                .topk(field("dist"), 3, false),
            None,
            None,
        )
        .await
        .expect_err("Query should fail");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
