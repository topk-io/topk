use std::collections::HashMap;
use test_context::test_context;
use topk_rs::{
    error::{CollectionValidationError, ValidationErrorBag},
    Error,
};

mod utils;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_list_collections(ctx: &mut ProjectTestContext) {
    let c = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let response = ctx
        .client
        .collections()
        .list()
        .await
        .expect("could not list collections");

    assert!(response.iter().any(|cc| cc == &c));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_collection(ctx: &mut ProjectTestContext) {
    let c = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let collections = ctx
        .client
        .collections()
        .list()
        .await
        .expect("could not list collections");

    assert!(collections.iter().any(|cc| cc == &c));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_collection_with_invalid_name(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .create(ctx.wrap("books with spaces"), HashMap::default())
        .await
        .expect_err("could not create collection");

    assert!(
        matches!(err, Error::CollectionValidationError(e) if e == ValidationErrorBag::new(vec![
            CollectionValidationError::InvalidName("must start with a letter or underscore and contain only letters, numbers, underscores, and dashes".to_string())
        ]))
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_create_duplicate_collection(ctx: &mut ProjectTestContext) {
    ctx.client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
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
        .create(ctx.wrap("test"), HashMap::default())
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
        .list()
        .await
        .expect("could not list collections");

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
        .create(ctx.wrap("test"), HashMap::default())
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
