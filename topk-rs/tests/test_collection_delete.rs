use std::collections::HashMap;
use test_context::test_context;
use topk_rs::data::literal;
use topk_rs::doc;
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

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_with_filter(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let collection = ctx.client.collection(&collection.name);

    let mut lsn = String::new();
    for batch_idx in 0..3 {
        lsn = collection
            .upsert(
                (0..5)
                    .map(|i| {
                        let idx = batch_idx * 5 + i;
                        doc!("_id" => format!("{}", idx), "batch_idx" => batch_idx)
                    })
                    .collect(),
            )
            .await
            .expect("could not upsert document");

        assert_eq!(lsn, format!("{}", batch_idx + 1));
    }
    assert_eq!(
        collection
            .count(Some(lsn.clone()), None)
            .await
            .expect("could not count documents"),
        15
    );

    // Delete using filter
    let lsn = collection
        .delete(field("batch_idx").gte(literal(1)))
        .await
        .expect("could not delete document");
    assert_eq!(lsn, "4");

    assert_eq!(
        collection
            .count(Some(lsn.clone()), None)
            .await
            .expect("could not count documents"),
        5
    );

    // Upsert more records. The upsert records satisfy the delete filter
    // but should not be deleted (snice the write happended after the delete).
    let lsn = collection
        .upsert(
            (0..5)
                .map(|i| doc!("_id" => format!("{}", 15 + i), "batch_idx" => 3, "updated" => true))
                .collect(),
        )
        .await
        .expect("could not upsert document");

    assert_eq!(lsn, "5");

    // Verify that only documents from batch_idx 1, 2 are deleted
    let doc_ids = collection
        .query(
            select([("_id", field("_id"))]).topk(field("batch_idx"), 100, true),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query documents");

    let expected_doc_ids: Vec<_> = (0..20)
        .filter(|i| (i / 5) < 1 || (i / 5) > 2)
        .map(|i| format!("{i}"))
        .collect();
    assert_doc_ids!(doc_ids, expected_doc_ids);

    // Delete updated documents
    let lsn = collection
        .delete(field("updated").eq(literal(true)))
        .await
        .expect("could not delete document");
    assert_eq!(lsn, "6");

    // Verify expected documents
    let doc_ids = collection
        .query(
            select([("_id", field("_id"))]).topk(field("batch_idx"), 100, true),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query documents");

    assert_doc_ids!(doc_ids, (0..5).map(|i| format!("{i}")));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_delete_with_invalid_filter(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let collection = ctx.client.collection(&collection.name);

    let mut lsn = String::new();
    for batch_idx in 0..3 {
        lsn = collection
            .upsert(
                (0..5)
                    .map(|i| {
                        let idx = batch_idx * 5 + i;
                        doc!("_id" => format!("{}", idx), "batch_idx" => batch_idx)
                    })
                    .collect(),
            )
            .await
            .expect("could not upsert document");

        assert_eq!(lsn, format!("{}", batch_idx + 1));
    }

    collection
        .delete(field("batch_idx").gte(literal(1)).or(field("batch_idx")))
        .await
        .expect_err("delete should fail with invalid filter");

    assert_eq!(
        collection
            .count(Some(lsn), None)
            .await
            .expect("could not count documents"),
        15
    );
}
