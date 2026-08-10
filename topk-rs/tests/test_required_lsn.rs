use test_context::test_context;
use topk_rs::query::{field, select};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_invalid_required_lsn(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;
    let collection = ctx.client.collection(&collection.name);

    let err = collection
        .get(["1984"], None, Some("0".to_string()), None)
        .await
        .expect_err("get with zero lsn must be rejected");
    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s.contains("Invalid required LSN")),
        "get: {err:?}"
    );

    let err = collection
        .query(
            select([("_id", field("_id"))]).limit(1),
            Some("0".to_string()),
            None,
        )
        .await
        .expect_err("query with zero lsn must be rejected");
    assert!(
        matches!(err, Error::InvalidArgument(ref s) if s.contains("Invalid required LSN")),
        "query: {err:?}"
    );
}
