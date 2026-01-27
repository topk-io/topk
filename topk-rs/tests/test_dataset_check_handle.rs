use std::collections::HashMap;
use test_context::test_context;
use topk_rs::Error;

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_check_handle_not_processed(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(
            "doc1".to_string().into(),
            &test_pdf_path(),
            HashMap::default(),
        )
        .await
        .expect("could not upsert file");

    let processed = ctx
        .client
        .dataset(&dataset.name)
        .check_handle(handle.clone(), false)
        .await
        .expect("could not check handle");

    // Handle should not be processed yet
    assert_eq!(processed, false);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_check_handle_wait_until_processed(ctx: &mut ProjectTestContext) {
    let pdf_path = test_pdf_path();

    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc2".to_string().into(), &pdf_path, HashMap::default())
        .await
        .expect("could not upsert file");

    let processed = ctx
        .client
        .dataset(&dataset.name)
        .check_handle(handle.clone(), true)
        .await
        .expect("could not check handle with wait");

    // Handle should be processed after waiting
    assert_eq!(processed, true);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_check_handle_invalid_handle(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let err = ctx
        .client
        .dataset(&dataset.name)
        .check_handle("invalid-handle-format-12345".to_string().into(), false)
        .await
        .expect_err("should not be able to check handle with invalid handle");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_check_handle_from_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .check_handle("some-handle".to_string().into(), false)
        .await
        .expect_err("should not be able to check handle for non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}
