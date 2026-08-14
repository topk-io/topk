use std::collections::HashMap;

use test_context::test_context;

use topk_rs::doc;
use topk_rs::proto::v1::ctx::file::InputFile;
use topk_rs::proto::v1::data::Value;
use topk_rs::Error;

mod utils;
use utils::{dataset::test_pdf, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), vec![("title", Value::string("A"))])
        .await
        .expect("could not upsert file");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for upsert handle");

    let handle = ctx
        .client
        .dataset(&dataset.name)
        .update_metadata(
            "doc1",
            vec![
                ("title", Value::string("B")),
                ("author", Value::string("X")),
            ],
        )
        .await
        .expect("could not update metadata");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for update handle");

    let docs = ctx
        .client
        .dataset(&dataset.name)
        .get_metadata(vec!["doc1"], None)
        .await
        .expect("could not get metadata");

    assert_eq!(
        docs,
        HashMap::from([(
            "doc1".to_string(),
            doc!(
                "title" => Value::string("B"),
                "author" => Value::string("X"),
            )
        )])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata_non_existent_document(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    let result = ctx
        .client
        .dataset(&dataset.name)
        .update_metadata("missing", vec![("title", Value::string("B"))])
        .await;

    assert!(matches!(result, Err(Error::DatasetNotFound)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata_with_invalid_fields(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    for field in ["_title", "topk.title"] {
        let result = ctx
            .client
            .dataset(&dataset.name)
            .update_metadata("doc1", vec![(field, Value::string("B"))])
            .await;

        assert!(
            matches!(result, Err(Error::DocumentValidationError(_))),
            "expected validation error for field {field:?}, got {result:?}"
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata_rejected_when_pending(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    // Upload, but don't wait for the handle — doc is still Pending.
    let _handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file("doc1", test_pdf(), Vec::<(String, Value)>::new())
        .await
        .expect("could not upsert file");

    let err = ctx
        .client
        .dataset(&dataset.name)
        .update_metadata("doc1", vec![("title", Value::string("B"))])
        .await
        .expect_err("expected rejection for Pending doc");

    assert!(
        err.to_string().contains("still being processed"),
        "got: {err:?}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata_rejected_when_in_error_state(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

    // Upload a corrupted PDF and wait for the pipeline to mark it as Error.
    let handle = ctx
        .client
        .dataset(&dataset.name)
        .upsert_file(
            "doc1",
            InputFile::from_bytes("doc1.pdf", b"not a real pdf".to_vec(), "application/pdf")
                .expect("could not build InputFile"),
            Vec::<(String, Value)>::new(),
        )
        .await
        .expect("could not upsert corrupted PDF");
    ctx.client
        .dataset(&dataset.name)
        .wait_for_handle(&handle, None)
        .await
        .expect("could not wait for handle");

    let err = ctx
        .client
        .dataset(&dataset.name)
        .update_metadata("doc1", vec![("title", Value::string("B"))])
        .await
        .expect_err("expected rejection for Error doc");

    assert!(
        err.to_string().contains("Document is in error state")
            && err.to_string().contains("Corrupted PDF"),
        "got: {err:?}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata_rejected_when_deleting(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None)
        .await
        .expect("could not create dataset");

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
        .expect("could not wait for upsert handle");

    // Kick off delete but don't wait — doc is in Deleting state.
    let _handle = ctx
        .client
        .dataset(&dataset.name)
        .delete("doc1")
        .await
        .expect("could not delete");

    let err = ctx
        .client
        .dataset(&dataset.name)
        .update_metadata("doc1", vec![("title", Value::string("B"))])
        .await
        .expect_err("expected rejection for Deleting/deleted doc");

    // The transform task may have already processed the delete, in which case
    // the doc is gone and we get NotFound. Either outcome proves the update
    // cannot proceed against a doc that's being/has been deleted.
    assert!(
        err.to_string().contains("being deleted") || matches!(err, Error::DatasetNotFound),
        "got: {err:?}"
    );
}
