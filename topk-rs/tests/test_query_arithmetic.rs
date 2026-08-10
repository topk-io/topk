use test_context::test_context;
use topk_rs::data::literal;
use topk_rs::query::{field, select};
use topk_rs::Error;

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_division_by_zero(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;
    let collection = ctx.client.collection(&collection.name);

    for expr in [
        field("published_year").div(literal(0u32)),
        // 1813 is the earliest published year, so the divisor is zero for that row only.
        field("published_year").div(field("published_year").sub(literal(1813u32))),
    ] {
        let err = collection
            .query(select([("q", expr.clone())]).limit(100), None, None)
            .await
            .expect_err("division by zero must be rejected");

        assert!(
            matches!(err, Error::InvalidArgument(ref s) if s.contains("Divide by zero")),
            "{expr:?} -> {err:?}"
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_arithmetic_overflow(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;
    let collection = ctx.client.collection(&collection.name);

    for expr in [
        field("published_year").mul(literal(4_000_000u32)),
        field("published_year").add(literal(u32::MAX)),
        field("published_year").sub(literal(u32::MAX)),
    ] {
        let err = collection
            .query(select([("q", expr.clone())]).limit(100), None, None)
            .await
            .expect_err("arithmetic overflow must be rejected");

        assert!(
            matches!(err, Error::InvalidArgument(ref s) if s.contains("overflow")),
            "{expr:?} -> {err:?}"
        );
    }
}
