use test_context::test_context;
use topk_protos::doc;
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_from_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .get("doc1", vec![], None, None)
        .await
        .expect_err("should not be able to get document from non-existent collection");

    // TODO: this should return `CollectionNotFound`
    assert!(matches!(err, Error::DocumentNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_non_existent_document(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .get("missing", vec![], None, None)
        .await
        .expect_err("should not be able to get non-existent document");

    assert!(matches!(err, Error::DocumentNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_document(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let lotr = dataset::books::docs()
        .into_iter()
        .find(|doc| doc.id().unwrap() == "lotr")
        .clone()
        .unwrap();

    let doc = ctx
        .client
        .collection(&collection.name)
        .get("lotr", vec![], None, None)
        .await
        .expect("could not get document");

    assert_eq!(doc, lotr);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_get_document_fields(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let doc = ctx
        .client
        .collection(&collection.name)
        .get(
            "lotr",
            vec!["title".to_string(), "published_year".to_string()],
            None,
            None,
        )
        .await
        .expect("could not get document");

    assert_eq!(
        doc,
        doc!("_id" => "lotr", "title" => "The Lord of the Rings: The Fellowship of the Ring", "published_year" => 1954 as u32)
    );
}
