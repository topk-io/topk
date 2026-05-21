use std::collections::HashMap;
use test_context::test_context;
use topk_rs::doc;
use topk_rs::proto::v1::data::Value;

mod utils;
use utils::{dataset::test_pdf, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_metadata(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None, None)
        .await
        .expect("could not create dataset");

    // Upsert file with metadata
    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(
            "doc1".to_string(),
            test_pdf(),
            vec![("title", Value::string("test"))],
        )
        .await
        .expect("could not upsert file");

    // Wait for file to be processed
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for handle");

    // Get metadata and verify it matches
    let docs = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .expect("could not get metadata");

    assert_eq!(
        docs,
        HashMap::from([("doc1".to_string(), doc!("title" => Value::string("test")))])
    );
}
