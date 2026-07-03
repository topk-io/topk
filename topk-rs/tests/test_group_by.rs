use test_context::test_context;
use topk_rs::proto::v1::data::AggregateExpr;
use topk_rs::query::{field, group_by};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_group_by_bool_key_expr(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            group_by(
                [("is_old", field("published_year").lt(1940 as u32))],
                [("count", AggregateExpr::count(None))],
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 2);

    for row in result {
        if row.fields["is_old"].as_bool().unwrap() {
            assert_eq!(row.fields["count"].as_u64().unwrap(), 4);
        } else {
            assert_eq!(row.fields["count"].as_u64().unwrap(), 6);
        }
    }
}
