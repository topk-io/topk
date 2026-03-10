use test_context::test_context;
use topk_rs::{proto::v1::data::Value, Error};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::test_pdf;

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_delete_document(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset")
        .into_inner()
        .dataset
        .unwrap();

    // Try to get document metadata
    let resp = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .expect("could not get metadata");
    assert!(resp.docs.is_empty());

    let upsert = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect("could not upsert file");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&upsert.handle, None)
        .await
        .expect("could not wait handle");

    // Try to get document metadata
    let resp = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .expect("could not get metadata");
    assert_eq!(
        resp.into_inner().docs.keys().collect::<Vec<_>>(),
        vec!["doc1"]
    );

    // Delete the document
    let delete = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc1")
        .await
        .expect("could not delete");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&delete.handle, None)
        .await
        .expect("could not wait handle");

    // Try to get document metadata
    let resp = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .unwrap();
    assert!(resp.docs.is_empty());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_delete_non_existent_document_returns_handle(ctx: &mut ProjectTestContext) {
    let response = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"))
        .await
        .expect("could not create dataset");

    let delete = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .delete("nonexistent")
        .await
        .expect("could not delete");

    // Deleting a non-existent document returns a handle
    let result = ctx
        .client
        .dataset(&response.dataset().unwrap().name)
        .wait_for_handle(&delete.handle, None)
        .await;
    assert!(matches!(result, Ok(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore]
async fn test_delete_from_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .delete("doc1")
        .await
        .expect_err("should not be able to delete from non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}
