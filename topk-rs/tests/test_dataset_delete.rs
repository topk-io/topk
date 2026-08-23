use futures_util::TryStreamExt;
use test_context::test_context;
use topk_rs::proto::v1::ctx::file::InputFile;
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
        .create(ctx.wrap("test"), None, None)
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
        .create(ctx.wrap("test"), None, None)
        .await
        .expect("could not create dataset");

    let result = ctx
        .client
        .dataset(&dataset.name)
        .delete("nonexistent")
        .await;

    assert!(matches!(result, Err(Error::DocumentNotFound(_))));
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
async fn test_delete_id_prefix_sibling(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None, None)
        .await
        .expect("could not create dataset");

    for (id, body) in [
        ("doc-010", "# Apples\n\nA document about apples."),
        ("doc-01", "# Oranges\n\nA document about oranges."),
    ] {
        let file = InputFile::from_bytes(id, body.as_bytes(), "text/markdown")
            .expect("could not create InputFile from memory");
        let handle = ctx
            .client
            .dataset(&dataset.name)
            .upsert_file(id.to_string(), file, Vec::<(String, Value)>::new())
            .await
            .expect("could not upsert file");
        ctx.client
            .dataset(&dataset.name)
            .wait_for_handle(&handle, None)
            .await
            .expect("could not wait handle");
    }

    // deleting `doc-01` must not touch `doc-010`
    let handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc-01")
        .await
        .expect("could not delete");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait handle");

    let docs = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc-010", "doc-01"], None)
        .await
        .expect("could not get metadata");
    assert!(docs.contains_key("doc-010"));
    assert!(!docs.contains_key("doc-01"));

    let results: Vec<_> = ctx
        .client
        .search("apples", [&dataset.name], 10, None, Vec::<String>::new())
        .await
        .expect("could not search")
        .try_collect()
        .await
        .expect("could not collect search results");
    assert!(results.iter().any(|r| r.doc_id == "doc-010"));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_already_deleted(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None, None)
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
    assert!(matches!(result, Err(Error::DocumentNotFound(_))));
}
