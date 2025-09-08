use std::collections::HashMap;
use test_context::test_context;
use topk_rs::doc;
use topk_rs::error::{DocumentValidationError, ValidationErrorBag};
use topk_rs::proto::v1::control::VectorDistanceMetric;
use topk_rs::proto::v1::data::Value;
use topk_rs::proto::v1::{
    control::{field_type::DataType, FieldSpec, FieldType, FieldTypeText},
    data::Document,
};
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
async fn test_upsert_max_doc_size(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(collection.name)
        .upsert(vec![
            doc!("_id" => "one", "payload" => "x".repeat(500 * 1024)), // 500KB, too large
            doc!("_id" => "two", "payload" => "xxx"),                  // ok
            doc!("_id" => "three", "payload" => "x".repeat(130 * 1024)), // 130KB, too large
            doc!("_id" => "four", "payload" => "x".repeat(126 * 1024)), // 126KB (plus overhead), ok
        ])
        .await
        .expect_err("should not be able to upsert a batch containing oversized documents");

    match err {
        Error::DocumentValidationError(ref s) => {
            let mut by_id: HashMap<&str, &DocumentValidationError> = HashMap::new();
            for err in s.iter() {
                if let DocumentValidationError::DocumentTooLarge { doc_id, .. } = err {
                    by_id.insert(doc_id.as_str(), err);
                }
            }

            assert_eq!(
                by_id.len(),
                2,
                "expected two DocumentTooLarge errors, got: {:?}",
                s
            );

            match by_id.get("one").unwrap() {
                DocumentValidationError::DocumentTooLarge {
                    doc_id,
                    max_size_bytes,
                    got_size_bytes,
                } => {
                    assert_eq!(doc_id, "one");
                    assert_eq!(*max_size_bytes, 128 * 1024);
                    assert!(
                        *got_size_bytes > *max_size_bytes,
                        "expected got_size_bytes > max_size_bytes"
                    );
                }
                other => panic!("unexpected error variant for 'one': {:?}", other),
            }

            match by_id.get("three").unwrap() {
                DocumentValidationError::DocumentTooLarge {
                    doc_id,
                    max_size_bytes,
                    got_size_bytes,
                } => {
                    assert_eq!(doc_id, "three");
                    assert_eq!(*max_size_bytes, 128 * 1024);
                    assert!(
                        *got_size_bytes > *max_size_bytes,
                        "expected got_size_bytes > max_size_bytes"
                    );
                }
                other => panic!("unexpected error variant for 'three': {:?}", other),
            }
        }
        _ => panic!("expected DocumentValidationError, got {:?}", err),
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_vectors(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from([
                (
                    "f32_vector".to_string(),
                    FieldSpec::f32_vector(4, false, VectorDistanceMetric::Cosine),
                ),
                (
                    "u8_vector".to_string(),
                    FieldSpec::u8_vector(3, false, VectorDistanceMetric::Cosine),
                ),
                (
                    "binary_vector".to_string(),
                    FieldSpec::binary_vector(2, false, VectorDistanceMetric::Hamming),
                ),
            ]),
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "x",
            "f32_vector" => vec![1.0, 2.0, 3.0, 4.0],
            "u8_vector" => Value::list(vec![4u8, 5u8, 6u8]),
            "binary_vector" => Value::list(vec![7u8, 8u8]),
        )])
        .await
        .expect("could not upsert document");

    let obj = ctx
        .client
        .collection(&collection.name)
        .get(vec!["x".to_string()], None, Some(lsn), None)
        .await
        .expect("could not get document");

    assert_eq!(
        obj["x"]["f32_vector"],
        Value::list(vec![1.0f32, 2.0, 3.0, 4.0])
    );
    assert_eq!(obj["x"]["u8_vector"], Value::list(vec![4u8, 5u8, 6u8]));
    assert_eq!(obj["x"]["binary_vector"], Value::list(vec![7u8, 8u8]));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_sparse_vectors(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(
            ctx.wrap("test"),
            HashMap::from_iter([
                (
                    "f32_sparse_vector".to_string(),
                    FieldSpec::f32_sparse_vector(true, VectorDistanceMetric::DotProduct),
                ),
                (
                    "u8_sparse_vector".to_string(),
                    FieldSpec::u8_sparse_vector(true, VectorDistanceMetric::DotProduct),
                ),
            ]),
        )
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![doc!(
            "_id" => "x",
            "f32_sparse_vector" => Value::f32_sparse_vector(vec![1, 2, 3], vec![1.2, 2.3, 3.4]),
            "u8_sparse_vector" => Value::u8_sparse_vector(vec![1, 2, 3], vec![4u8, 5u8, 6u8]),
        )])
        .await
        .expect("could not upsert document");

    let obj = ctx
        .client
        .collection(&collection.name)
        .get(vec!["x".to_string()], None, Some(lsn), None)
        .await
        .expect("could not get document");

    assert_eq!(
        obj["x"]["f32_sparse_vector"],
        Value::f32_sparse_vector(vec![1, 2, 3], vec![1.2, 2.3, 3.4]),
    );
    assert_eq!(
        obj["x"]["u8_sparse_vector"],
        Value::u8_sparse_vector(vec![1, 2, 3], vec![4u8, 5u8, 6u8]),
    );
}
