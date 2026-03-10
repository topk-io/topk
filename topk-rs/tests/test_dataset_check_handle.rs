use std::collections::HashMap;

use test_context::test_context;
use topk_rs::{proto::v1::data::Value, Error};

mod utils;
use utils::{dataset::test_pdf, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_wait_for_handle(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset")
        .into_inner()
        .dataset
        .unwrap();

    let upsert = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), HashMap::<String, Value>::default())
        .await
        .expect("could not upsert file");

    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&upsert.handle, None)
        .await
        .expect("handle was not processed within timeout");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_check_handle_invalid_handle(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let err = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .check_handle("invalid-handle-format-12345")
        .await
        .expect_err("should not be able to check handle with invalid handle");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_check_handle_from_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .check_handle("some-handle")
        .await
        .expect_err("should not be able to check handle for non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}
