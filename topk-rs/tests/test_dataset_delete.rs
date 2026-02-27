use test_context::test_context;
use topk_rs::{
    proto::v1::{ctx::file::InputFile, data::Value},
    Error,
};

mod utils;
use utils::{dataset::test_pdf_path, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_document(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let _handle = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .upsert_file(
            "doc1",
            InputFile::from_path(test_pdf_path()).expect("could not create InputFile from path"),
            Vec::<(String, Value)>::new(),
        )
        .await
        .expect("could not upsert file");

    // Delete the document
    let delete_handle = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .delete("doc1")
        .await;

    assert!(matches!(delete_handle, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_non_existent_document_returns_handle(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let delete_handle = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .delete("nonexistent")
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
        .delete("doc1".to_string())
        .await
        .expect_err("should not be able to delete from non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_returns_handle(ctx: &mut ProjectTestContext) {
    let pdf_path = test_pdf_path();

    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    // Upload a document
    let _upsert_handle = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .upsert_file(
            "doc2",
            InputFile::from_path(pdf_path).expect("could not create InputFile from path"),
            Vec::<(String, Value)>::new(),
        )
        .await
        .expect("could not upsert file");

    // Delete and verify handle is returned
    let response = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .delete("doc2")
        .await
        .expect("should delete successfully");

    assert_eq!(response.handle.is_empty(), false);
}
