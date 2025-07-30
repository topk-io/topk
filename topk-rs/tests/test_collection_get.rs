use std::collections::HashMap;

use test_context::test_context;
use topk_rs::doc;
use topk_rs::proto::v1::data::ConsistencyLevel;
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
