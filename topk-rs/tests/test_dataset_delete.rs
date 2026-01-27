use std::collections::HashMap;
use test_context::test_context;
use topk_rs::Error;

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_document(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let _handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(
            "doc1".to_string().into(),
            &test_pdf_path(),
            HashMap::default(),
        )
        .await
        .expect("could not upsert file");

    // Delete the document
    let delete_handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc1".to_string().into())
        .await;

    assert!(matches!(delete_handle, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_non_existent_document_returns_handle(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let delete_handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("nonexistent".to_string().into())
        .await;

    // Deleting a non-existent document returns a handle
    assert!(matches!(delete_handle, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_from_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .delete("doc1".to_string().into())
        .await
        .expect_err("should not be able to delete from non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_returns_handle(ctx: &mut ProjectTestContext) {
    let pdf_path = test_pdf_path();

    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    // Upload a document
    let _upsert_handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc2".to_string().into(), &pdf_path, HashMap::default())
        .await
        .expect("could not upsert file");

    // Delete and verify handle is returned
    let delete_handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc2".to_string().into())
        .await;

    let handle: String = delete_handle.expect("should delete successfully").into();
    assert_eq!(handle.is_empty(), false);
}
