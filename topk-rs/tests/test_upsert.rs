use std::collections::HashMap;
use test_context::test_context;
use topk_protos::doc;
use topk_protos::v1::{
    control::{field_type::DataType, FieldSpec, FieldType, FieldTypeText},
    data::Document,
};
use topk_rs::error::{DocumentValidationError, ValidationErrorBag};
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_to_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .upsert(vec![doc!("_id" => "one")])
        .await
        .expect_err("should not be able to upsert document to non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_basic(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!("_id" => "one")])
        .await
        .expect("could not upsert document");

    assert_eq!(&lsn, "1");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_batch(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(collection.name)
        .upsert(vec![doc!("_id" => "one"), doc!("_id" => "two")])
        .await
        .expect("could not upsert document");

    assert_eq!(&lsn, "1");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_sequential(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!("_id" => "one")])
        .await
        .expect("could not upsert document");
    assert_eq!(&lsn, "1");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!("_id" => "two")])
        .await
        .expect("could not upsert document");
    assert_eq!(&lsn, "2");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!("_id" => "three")])
        .await
        .expect("could not upsert document");
    assert_eq!(&lsn, "3");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_no_documents(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(collection.name)
        .upsert(vec![])
        .await
        .expect_err("should not be able to upsert invalid document");

    assert!(
        matches!(err, Error::DocumentValidationError(ref s) if s == &ValidationErrorBag::from(vec![DocumentValidationError::NoDocuments {}])),
        "got error: {:?}",
        err
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_invalid_document(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(collection.name)
        .upsert(vec![Document::default()])
        .await
        .expect_err("should not be able to upsert invalid document");

    assert!(
        matches!(
            err,
            Error::DocumentValidationError(ref s) if s == &ValidationErrorBag::from(vec![DocumentValidationError::MissingId { doc_offset: 0 } ])
        ),
        "got error: {:?}",
        err
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_schema_validation(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([(
                "name".to_string(),
                FieldSpec {
                    data_type: Some(FieldType {
                        data_type: Some(DataType::Text(FieldTypeText {})),
                    }),
                    required: true,
                    index: None,
                },
            )]),
        )
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(collection.name)
        .upsert(vec![doc!("_id" => "one")])
        .await
        .expect_err("should not be able to upsert invalid document");

    assert!(
        matches!(
            err,
            Error::DocumentValidationError(ref s) if s == &ValidationErrorBag::from(vec![DocumentValidationError::MissingField {
                field: "name".to_string(),
                doc_id: "one".to_string(),
            }])
        ),
        "got error: {:?}",
        err
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_too_fast(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    async fn send_upsert(
        client: topk_rs::Client,
        collection: &str,
        size_mb: usize,
    ) -> Result<String, Error> {
        client
            .collection(collection)
            .upsert(
                (0..size_mb)
                    .map(|_| doc!("_id" => "one", "value" => "x".repeat(1024 * 1024)))
                    .collect(),
            )
            .await
    }

    let results = tokio::join!(
        send_upsert(ctx.client.clone(), &collection.name, 5),
        send_upsert(ctx.client.clone(), &collection.name, 5),
        send_upsert(ctx.client.clone(), &collection.name, 5),
        send_upsert(ctx.client.clone(), &collection.name, 5),
        send_upsert(ctx.client.clone(), &collection.name, 5),
        send_upsert(ctx.client.clone(), &collection.name, 5),
    );

    // Assert that at least one of the requests returned a SlowDown error
    assert!(matches!(
        results,
        (Err(Error::SlowDown(_)), _, _, _, _, _)
            | (_, Err(Error::SlowDown(_)), _, _, _, _)
            | (_, _, Err(Error::SlowDown(_)), _, _, _)
            | (_, _, _, Err(Error::SlowDown(_)), _, _)
            | (_, _, _, _, Err(Error::SlowDown(_)), _)
            | (_, _, _, _, _, Err(Error::SlowDown(_)))
    ));
}
