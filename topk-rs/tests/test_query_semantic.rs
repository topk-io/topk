use test_context::test_context;
use topk_rs::{
    query::{field, fns, r#match, select},
    Error,
};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_schema(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    for (field, _) in collection.schema.iter() {
        assert!(
            !field.starts_with("_"),
            "Schema contains reserved field: {}",
            field
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_write_docs(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .count(None, None)
        .await
        .expect("could not query");

    assert_eq!(result, 10_u64);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("sim", fns::semantic_similarity("title", "dummy"))]).sort(field("sim"), true).limit(3),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 3);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_with_text_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("sim", fns::semantic_similarity("title", "dummy"))])
                .filter(r#match("love", Some("summary"), None, false))
                .sort(field("sim"), true).limit(3),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["gatsby", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_with_missing_index(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("sim", fns::semantic_similarity("published_year", "dummy"))]).sort(field("sim"), true).limit(3),
            None,
            None,
        )
        .await
        .expect_err("should not query");

    assert!(matches!(err, Error::InvalidArgument(_)));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_semantic_index_query_multiple_fields(ctx: &mut ProjectTestContext) {
    let collection = dataset::semantic::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("title_sim", fns::semantic_similarity("title", "dummy")),
                ("summary_sim", fns::semantic_similarity("summary", "query")),
            ])
            .sort(field("title_sim").add(field("summary_sim")), true).limit(5),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 5);
}
