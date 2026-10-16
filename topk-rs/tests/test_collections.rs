use std::collections::HashMap;

use futures::TryStreamExt;
use test_context::test_context;
use topk_rs::{
    error::{SchemaValidationError, ValidationErrorBag},
    proto::v1::control::{
        field_type_matrix::MatrixValueType, FieldIndex, FieldSpec, MultiVectorQuantization,
        VectorDistanceMetric,
    },
    Error,
};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::multi_vec::schema;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_collections(ctx: &mut ProjectTestContext) {
    let c = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let response = ctx
        .client
        .collections()
        .list(None)
        .await
        .expect("could not list collections")
        .try_collect::<Vec<_>>()
        .await
        .expect("could not receive collections from stream");

    assert!(response.iter().any(|cc| cc == &c));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_collections_with_prefix(ctx: &mut ProjectTestContext) {
    let names = ["a", "ab", "b"].map(|name| ctx.wrap(name));
    for name in &names {
        ctx.client
            .collections()
            .create(name, HashMap::default(), None)
            .await
            .expect("could not create collection");
    }

    for (prefix, expected) in [
        (ctx.scope.clone(), &names[..]),
        (ctx.wrap("a"), &names[..2]),
    ] {
        let listed = ctx
            .client
            .collections()
            .list(Some(prefix))
            .await
            .expect("could not list collections")
            .map_ok(|c| c.name)
            .try_collect::<Vec<_>>()
            .await
            .expect("could not receive collections from stream");
        assert_eq!(listed, expected);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_collections_with_invalid_prefix(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .list(Some("-books".into()))
        .await
        .err()
        .expect("should not be able to list with invalid prefix");

    assert!(matches!(err, Error::InvalidArgument(_)), "{err:?}");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_collection(ctx: &mut ProjectTestContext) {
    let c = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let collections = ctx
        .client
        .collections()
        .list(None)
        .await
        .expect("could not list collections")
        .try_collect::<Vec<_>>()
        .await
        .expect("could not receive collections from stream");

    assert!(collections.iter().any(|cc| cc == &c));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_collection_with_invalid_name(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .create(ctx.wrap("books with spaces"), HashMap::default(), None)
        .await
        .expect_err("could not create collection");

    assert!(matches!(
        err,
        topk_rs::Error::InvalidArgument(e) if e.contains("Invalid collection name")
    ));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_duplicate_collection(ctx: &mut ProjectTestContext) {
    ctx.client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect_err("should not be able to create duplicate collection");

    assert!(matches!(err, Error::CollectionAlreadyExists));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .delete(ctx.wrap("test"))
        .await
        .expect_err("should not be able to delete non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_collection(ctx: &mut ProjectTestContext) {
    let c = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    ctx.client
        .collections()
        .delete(ctx.wrap("test"))
        .await
        .expect("could not delete collection");

    let collections = ctx
        .client
        .collections()
        .list(None)
        .await
        .expect("could not list collections")
        .try_collect::<Vec<_>>()
        .await
        .expect("could not receive collections from stream");

    assert!(!collections.iter().any(|cc| *cc == c));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_collection(ctx: &mut ProjectTestContext) {
    // Test getting non-existent collection
    let err = ctx
        .client
        .collections()
        .get(ctx.wrap("test"))
        .await
        .expect_err("should not be able to get non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));

    // Create collection
    let c = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    // Get collection
    let collection = ctx
        .client
        .collections()
        .get(ctx.wrap("test"))
        .await
        .expect("could not get collection");

    assert_eq!(collection, c);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_collection_with_invalid_vector_dimension(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from([(
                "high_dim_vector".to_string(),
                FieldSpec::f32_vector(20_000, false)
                    .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine)),
            )]),
            None,
        )
        .await
        .expect_err("should fail");

    assert!(
        matches!(err, Error::SchemaValidationError(e) if e == ValidationErrorBag::new(vec![
            SchemaValidationError::VectorDimensionTooLarge {
                field: "high_dim_vector".to_string(),
                dimension: 20_000,
            }
        ]))
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_collection_with_invalid_multi_vector_quant(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .create(
            ctx.wrap("multi_vec"),
            schema(
                MatrixValueType::U8,
                Some(MultiVectorQuantization::Scalar),
                None,
                None,
            ),
            None,
        )
        .await
        .expect_err("should fail to create collection");

    assert!(matches!(err, Error::SchemaValidationError(_)));
}
