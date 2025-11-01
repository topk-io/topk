use std::collections::HashMap;
use test_context::test_context;

use topk_rs::doc;
use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, fns, select};
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;

use crate::utils::dataset;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_non_existent_collection(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collection("missing")
        .update(vec![doc!("_id" => "one")])
        .await
        .expect_err("should not be able to upsert document to non-existent collection");

    assert!(matches!(err, Error::CollectionNotFound), "got {:?}", err);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_batch(ctx: &mut ProjectTestContext) {
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
            doc!("_id" => "1", "foo" => "bar1"),
            doc!("_id" => "2", "foo" => "bar2"),
            doc!("_id" => "3", "foo" => "bar3"),
            doc!("_id" => "4", "foo" => "bar4"),
        ])
        .await
        .expect("could not upsert document");

    assert_eq!(&lsn, "1");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .update(vec![
            doc!("_id" => "2", "foo" => "bar2.2", "baz" => "foo"),
            doc!("_id" => "3", "foo" => Value::null()),
            doc!("_id" => "4", "foo" => "bar4.2"),
            doc!("_id" => "5", "foo" => "bar5"), // missing id
        ])
        .await
        .expect("could not update document");

    assert_eq!(&lsn, "2");

    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["1", "2", "3", "4", "5"], None, Some(lsn), None)
        .await
        .expect("could not get documents");

    assert_eq!(docs.len(), 4);
    assert_eq!(docs["1"], doc!("_id" => "1", "foo" => "bar1").fields);
    assert_eq!(
        docs["2"],
        doc!("_id" => "2", "foo" => "bar2.2", "baz" => "foo").fields
    );
    assert_eq!(docs["3"], doc!("_id" => "3").fields);
    assert_eq!(docs["4"], doc!("_id" => "4", "foo" => "bar4.2").fields);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_missing_id(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    // Upsert some docs
    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "1", "foo" => "bar1"),
            doc!("_id" => "2", "foo" => "bar2"),
        ])
        .await
        .expect("could not upsert document");

    assert_eq!(&lsn, "1");

    // Update non-existent doc
    let new_lsn = ctx
        .client
        .collection(&collection.name)
        .update(vec![doc!("_id" => "3", "foo" => "bar3")])
        .await
        .expect("could not update document");

    assert!(new_lsn.is_empty());

    // Check that no changes were made
    let docs = ctx
        .client
        .collection(&collection.name)
        .get(["1", "2", "3"], None, Some(lsn), None)
        .await
        .expect("could not get documents");

    assert_eq!(docs.len(), 2);
    assert_eq!(docs["1"], doc!("_id" => "1", "foo" => "bar1").fields);
    assert_eq!(docs["2"], doc!("_id" => "2", "foo" => "bar2").fields);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_vector_index_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let res = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "dist",
                fns::vector_distance("summary_embedding", vec![2.0; 16]),
            )])
            .filter(field("_id").eq("1984"))
            .limit(1),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].fields["dist"], Value::f32(0.0));

    let lsn = ctx
        .client
        .collection(&collection.name)
        .update(vec![
            doc!("_id" => "1984", "summary_embedding" => vec![8.0; 16]),
        ])
        .await
        .expect("could not update document");

    let res = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "dist",
                fns::vector_distance("summary_embedding", vec![2.0; 16]),
            )])
            .filter(field("_id").eq("1984"))
            .limit(1),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(res.len(), 1);
    assert_eq!(res[0].fields["dist"], Value::f32(f32::powi(6.0, 2) * 16.0));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_semantic_index_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;
    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("sim", fns::semantic_similarity("title", "dummy"))]).topk(
                field("sim"),
                1,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 1);
    let id = result[0].id().unwrap();

    let lsn = ctx
        .client
        .collection(&collection.name)
        .update(vec![doc!("_id" => id, "title" => "foobarbaz")])
        .await
        .expect("could not update document");

    let updated = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .select([("sim", fns::semantic_similarity("title", "dummy"))])
                .filter(field("_id").eq(id))
                .limit(1),
            Some(lsn),
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].id().unwrap(), id);
    assert_eq!(updated[0].fields["title"], Value::string("foobarbaz"));
    assert_ne!(updated[0].fields["sim"], result[0].fields["sim"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_invalid_data_type(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .update(vec![doc!("_id" => "1984", "title" => 1984u32)])
        .await
        .expect_err("should fail to update with invalid data type");

    assert!(matches!(err, Error::DocumentValidationError(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_missing_required_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .update(vec![doc!("_id" => "1984", "title" => Value::null())])
        .await
        .expect_err("should fail to update with missing required field");

    assert!(matches!(err, Error::DocumentValidationError(_)));
}
