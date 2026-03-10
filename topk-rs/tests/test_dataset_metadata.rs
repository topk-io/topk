use std::collections::HashMap;
use test_context::test_context;
use topk_rs::proto::v1::data::Value;

mod utils;
use utils::{dataset::{test_pdf, quick_wait}, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_metadata(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let original_metadata = HashMap::from([("title".to_string(), Value::string("test"))]);

    // Upsert file with metadata
    let upsert = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .upsert_file("doc1".to_string(), test_pdf(), original_metadata.clone())
        .await
        .expect("could not upsert file");

    // Wait for file to be processed
    ctx.client
        .dataset(&response.dataset().unwrap().name)
        .wait_for_handle(&upsert.handle, quick_wait())
        .await
        .expect("could not wait for handle");

    // Get metadata and verify it matches
    let response = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .get_metadata("doc1".to_string(), None)
        .await
        .expect("could not get metadata");

    assert_eq!(
        response.metadata.get("title"),
        original_metadata.get("title")
    );
}
