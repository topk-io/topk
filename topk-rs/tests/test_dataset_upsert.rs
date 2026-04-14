use std::collections::HashMap;

use futures_util::TryStreamExt;
use rstest::rstest;
use test_context::test_context;
use test_context::AsyncTestContext;
use topk_rs::proto::v1::ctx::file::InputFile;
use topk_rs::proto::v1::data::Value;
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::{test_file, test_pdf};

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_upsert_file_to_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect_err("should not be able to upsert file to non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_upsert_file_pdf(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let metadata = HashMap::from([
        ("title".to_string(), Value::string("Test PDF")),
        ("author".to_string(), Value::string("Test Author")),
    ]);

    let response = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .upsert_file("doc1", test_pdf(), metadata)
        .await
        .expect("could not upsert PDF file");

    assert_eq!(response.handle.is_empty(), false);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_upsert_file_markdown(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let metadata = HashMap::from([("title".to_string(), Value::string("Test Markdown"))]);

    let file_data = b"# Test Markdown\n\nThis is a test markdown file.";
    let input_file = InputFile::from_bytes("doc_1", file_data.as_slice(), "text/markdown")
        .expect("could not create InputFile from memory");

    let handle = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .upsert_file("doc2".to_string(), input_file, metadata)
        .await;

    assert!(matches!(handle, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_upsert_file_with_invalid_metadata(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let file_data = b"# Test Markdown\n\nThis is a test markdown file.";
    let input_file = InputFile::from_bytes("doc_1", file_data.as_slice(), "text/markdown")
        .expect("could not create InputFile from memory");

    for field in ["_title", "topk.title"] {
        let metadata = HashMap::from([(field.to_string(), Value::string("Test Markdown"))]);

        let handle = ctx
            .client
            .dataset(&response.dataset().unwrap().name)
            .upsert_file("doc2".to_string(), input_file.clone(), metadata)
            .await;

        assert!(matches!(handle, Err(Error::DocumentValidationError(_))));
    }
}

#[rstest]
#[case("pdfko.pdf")]
#[case("markdown.md")]
#[case("pictures/sample.jpg")]
#[case("pictures/sample.png")]
#[case("pictures/sample.bmp")]
#[case("pictures/sample.gif")]
#[case("pictures/sample.tiff")]
#[case("pictures/sample.webp")]
#[tokio::test]
#[ignore]
async fn test_upsert_file(#[case] file: &str) {
    let ctx = ProjectTestContext::setup().await;

    // Create dataset
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset")
        .into_inner()
        .dataset
        .unwrap();

    // Upsert  file
    let upsert = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(file, test_file(file), HashMap::<String, Value>::new())
        .await
        .expect(&format!("could not upsert file: {file}"));

    // Wait for handle to be processed
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&upsert.handle, None)
        .await
        .expect("handle was not processed within timeout");

    // Search and verify we get the file back
    let results = ctx
        .client
        .search(file, [&dataset.name], 1, None, Vec::<String>::new())
        .await
        .expect("could not search")
        .into_inner()
        .try_collect::<Vec<_>>()
        .await
        .expect("could not collect search results");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].doc_id, file);

    ctx.teardown().await;
}
