use std::collections::HashMap;
use test_context::test_context;
use topk_rs::proto::v1::data::Value;

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_metadata(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let original_metadata = HashMap::from([("title".to_string(), Value::string("test"))]);

    // Upsert file with metadata
    let _handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(
            "doc1".to_string().into(),
            &test_pdf_path(),
            original_metadata.clone(),
        )
        .await
        .expect("could not upsert file");

    // Get metadata and verify it matches
    let retrieved_metadata = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata("doc1".to_string().into())
        .await
        .expect("could not get metadata");

    assert_eq!(
        retrieved_metadata.get("title"),
        original_metadata.get("title")
    );
}
