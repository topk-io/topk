use std::collections::HashMap;

use test_context::test_context;

use topk_rs::doc;
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
        .create(ctx.wrap("test"), None, None)
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
        .create(ctx.wrap("test"), None, None)
        .await
        .expect("could not create dataset");

    let result = ctx
        .client
        .dataset(&dataset.name)
        .update_metadata("missing", vec![("title", Value::string("B"))])
        .await;

    assert!(matches!(result, Err(Error::DocumentNotFound)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_metadata_with_invalid_fields(ctx: &mut ProjectTestContext) {
    let dataset = ctx
        .client
        .datasets()
        .create(ctx.wrap("test"), None, None)
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
