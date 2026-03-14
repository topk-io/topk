use std::collections::HashMap;
use test_context::test_context;
use topk_rs::query::{field, select};

mod utils;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_tombstone_only(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    // Delete non-existent IDs to create tombstone-only WAL segments.
    let lsn = ctx
        .client
        .collection(&collection.name)
        .delete(vec!["ghost_1".into(), "ghost_2".into()])
        .await
        .expect("could not delete");

    // Query should return empty results, not error.
    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("x", field("x"))]).topk(field("x"), 10, true),
            Some(lsn),
            None,
        )
        .await
        .expect("query on tombstone-only WAL should not fail");

    assert_eq!(results.len(), 0);
}
