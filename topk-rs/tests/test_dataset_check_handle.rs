use std::collections::HashMap;
use std::time::Duration;
use test_context::test_context;
use topk_rs::Error;

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_check_handle_waits_until_processed(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1".to_string(), test_pdf_path(), HashMap::default())
        .await
        .expect("could not upsert file");

    // Poll check_handle every second, wait up to 120 seconds
    let max_attempts = 120;
    let mut processed = false;
    for _ in 0..max_attempts {
        processed = ctx
            .client
            .dataset(&dataset.name)
            .check_handle(handle.clone())
            .await
            .expect("could not check handle");

        if processed {
            break;
        }

        // Sleep 1s at the end of each iteration
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // Handle should be processed after waiting
    assert_eq!(
        processed, true,
        "Handle was not processed within 30 seconds"
    );
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
        .check_handle("invalid-handle-format-12345".to_string().into())
        .await
        .expect_err("should not be able to check handle with invalid handle");

    assert!(matches!(err, Error::Internal(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_check_handle_from_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .check_handle("some-handle".to_string().into())
        .await
        .expect_err("should not be able to check handle for non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}
