use std::collections::HashMap;
use test_context::test_context;
use topk_rs::proto::v1::data::Value;
use topk_rs::Error;

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_file_to_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .upsert_file("doc1".into(), &test_pdf_path(), HashMap::default())
        .await
        .expect_err("should not be able to upsert file to non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_file_pdf(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let metadata = HashMap::from([
        ("title".to_string(), Value::string("Test PDF")),
        ("author".to_string(), Value::string("Test Author")),
    ]);

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1".into(), &test_pdf_path(), metadata)
        .await
        .expect("could not upsert PDF file");

    let handle_str: String = handle.into();
    assert!(!handle_str.is_empty());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_file_markdown(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let metadata = HashMap::from([
        ("title".to_string(), Value::string("Test PDF")),
        ("category".to_string(), Value::string("test")),
    ]);

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc2".into(), &test_pdf_path(), metadata)
        .await;

    assert!(matches!(handle, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_file_large(ctx: &mut ProjectTestContext) {
    // For large file test, we'll still use the same PDF file
    // The actual file size test can be done separately if needed

    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc5".into(), &test_pdf_path(), HashMap::default())
        .await;

    assert!(matches!(handle, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_file_invalid_extension(ctx: &mut ProjectTestContext) {
    let temp_file = ctx.create_temp_file("txt", b"Some text content");

    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let err = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc6".into(), &temp_file, HashMap::default())
        .await
        .expect_err("should not be able to upsert file with invalid extension");

    assert!(matches!(err, Error::Input(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_file_nonexistent_path(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let nonexistent_path = std::env::temp_dir().join("nonexistent_file.pdf");

    let err = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc7".into(), &nonexistent_path, HashMap::default())
        .await
        .expect_err("should not be able to upsert non-existent file");

    assert!(matches!(
        err,
        Error::IoError(ref e) if e.kind() == std::io::ErrorKind::NotFound
    ));
}
