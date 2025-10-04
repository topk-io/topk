use std::collections::HashMap;

use test_context::test_context;
use topk_rs::data::literal;
use topk_rs::doc;
use topk_rs::proto::v1::data::ConsistencyLevel;
use topk_rs::query::field;
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

use crate::utils::dataset::Dataset;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_from_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .get(["doc1"], None, None, None)
        .await
        .expect_err("should not be able to get document from non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_non_existent_document(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["missing"], None, None, None)
        .await
        .expect("get failed");

    assert_eq!(docs, HashMap::new());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_document(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let docs = dataset::books::docs();
    let lotr = docs.find_by_id("lotr").unwrap().clone();

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["lotr"], None, None, None)
        .await
        .expect("could not get document");

    assert_eq!(docs, HashMap::from([("lotr".to_string(), lotr.fields)]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_multiple_documents(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["lotr", "moby"], None, None, None)
        .await
        .expect("could not get documents");

    let books = dataset::books::docs();
    let lotr = books.find_by_id("lotr").unwrap().clone();
    let moby = books.find_by_id("moby").unwrap().clone();

    assert_eq!(
        docs,
        HashMap::from([
            ("lotr".to_string(), lotr.fields),
            ("moby".to_string(), moby.fields)
        ])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_document_fields(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(
            ["lotr"],
            Some(vec!["title".to_string(), "published_year".to_string()]),
            None,
            None,
        )
        .await
        .expect("could not get document");

    assert_eq!(
        docs,
        HashMap::from([(
            "lotr".to_string(),
            doc!(
                "_id" => "lotr",
                "title" => "The Lord of the Rings: The Fellowship of the Ring",
                "published_year" => 1954 as u32
            )
            .fields
        )])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_updated_document(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let mut lotr = dataset::books::docs().find_by_id("lotr").unwrap().clone();

    // Update document
    lotr.fields
        .insert("published_year".to_string(), 2025.into());

    ctx.client
        .collection(&collection.name)
        .upsert(vec![lotr.clone()])
        .await
        .expect("could not upsert document");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["lotr"], None, None, Some(ConsistencyLevel::Strong))
        .await
        .expect("could not get document");

    assert_eq!(docs, HashMap::from([("lotr".to_string(), lotr.fields)]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_deleted_document(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    ctx.client
        .collection(&collection.name)
        .delete(vec!["lotr".to_string()])
        .await
        .expect("could not upsert document");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["lotr"], None, None, Some(ConsistencyLevel::Strong))
        .await
        .expect("could not get document");

    assert_eq!(docs, HashMap::new());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_with_delete_filter(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let collection = ctx.client.collection(&collection.name);

    for batch_idx in 0..3 {
        let lsn = collection
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

    // Delete using filter
    let lsn = collection
        .delete(field("batch_idx").gte(literal(1)))
        .await
        .expect("could not delete document");
    assert_eq!(lsn, "4");

    // Get documents
    let docs = collection
        .get(["2", "8", "13"], None, Some(lsn.clone()), None)
        .await
        .expect("could not get documents");

    assert_eq!(docs.len(), 1, "{docs:?}");
    assert_eq!(
        docs.get("2").expect("document not found"),
        &doc!("_id" => "2", "batch_idx" => 0).fields
    );

    // Upsert more records. The upsert records satisfy the delete filter
    // but should not be deleted (snice the write happended after the delete).
    let lsn = collection
        .upsert(
            (10..15)
                .map(|i| doc!("_id" => format!("{}", i), "batch_idx" => 2, "updated" => true))
                .collect(),
        )
        .await
        .expect("could not upsert document");
    assert_eq!(lsn, "5");

    // Get documents
    let docs = collection
        .get(["2", "8", "13"], None, Some(lsn.clone()), None)
        .await
        .expect("could not get documents");

    assert_eq!(docs.len(), 2, "{docs:?}");
    assert!(docs.get("8").is_none());
    assert_eq!(
        docs.get("2").expect("document not found"),
        &doc!("_id" => "2", "batch_idx" => 0).fields
    );
    assert_eq!(
        docs.get("13").expect("document not found"),
        &doc!("_id" => "13", "batch_idx" => 2, "updated" => true).fields
    );
}
