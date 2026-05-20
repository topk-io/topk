use test_context::test_context;
use topk_rs::{proto::v1::data::Value, Error};

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset::test_pdf;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_document(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    // Try to get document metadata
    let docs = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .expect("could not get metadata");
    assert!(docs.is_empty());

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect("could not upsert file");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait handle");

    // Try to get document metadata
    let docs = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .expect("could not get metadata");
    assert_eq!(docs.keys().collect::<Vec<_>>(), vec!["doc1"]);

    // Delete the document
    let handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc1")
        .await
        .expect("could not delete");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait handle");

    // Try to get document metadata
    let docs = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .unwrap();
    assert!(docs.is_empty());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_non_existent_document(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let result = ctx
        .client
        .dataset(&dataset.name)
        .delete("nonexistent")
        .await;

    assert!(matches!(result, Err(Error::DatasetNotFound)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_from_non_existent_dataset(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .dataset(ctx.wrap("nonexistent"))
        .delete("doc1")
        .await
        .expect_err("should not be able to delete from non-existent dataset");

    assert!(matches!(err, Error::DatasetNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_already_deleted(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    ctx.client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect("could not upsert file");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc1")
        .await
        .expect("could not delete");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for delete handle");

    let result = ctx.client.dataset(&dataset.name).delete("doc1").await;
    assert!(matches!(result, Err(Error::DatasetNotFound)));
}
