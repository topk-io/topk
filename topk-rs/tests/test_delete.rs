use std::collections::HashMap;
use test_context::test_context;
use topk_protos::doc;
use topk_rs::query::{field, select};
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_from_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .delete(vec!["one".to_string()])
        .await
        .expect_err("should not be able to delete document from non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_document(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "one", "rank" => 1),
            doc!("_id" => "two", "rank" => 2),
        ])
        .await
        .expect("could not upsert document");
    assert_eq!(&lsn, "1");

    // wait for write to be flushed
    ctx.client
        .collection(&collection.name)
        .count(None, None)
        .await
        .expect("could not query documents");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .delete(vec!["one".to_string()])
        .await
        .expect("could not delete document");
    assert_eq!(&lsn, "2");

    let docs = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("rank"), 100, true),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query documents");

    assert_doc_ids!(docs, ["two"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_non_existent_document(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    // we can delete a non-existent document, and it will be ignored
    let lsn = ctx
        .client
        .collection(&collection.name)
        .delete(vec!["one".to_string()])
        .await
        .expect("could not delete document");
    assert_eq!(&lsn, "1");
}
