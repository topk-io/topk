use std::collections::HashSet;
use test_context::test_context;
use topk_rs::data::Vector;
use topk_rs::query::literal;
use topk_rs::query::{field, fns, r#match, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_unified(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "summary_distance",
                    fns::vector_distance("summary_embedding", Vector::F32(vec![2.0; 16])),
                ),
                ("bm25_score", fns::bm25_score()),
            ])
            .filter(r#match("love", None, Some(30.0)).or(r#match("young", None, Some(10.0))))
            .topk(
                field("bm25_score") + (field("summary_distance") * literal(100)),
                2,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert!(result.len() == 2);
    assert_eq!(
        result
            .into_iter()
            .map(|d| d.id().unwrap().to_string())
            .collect::<HashSet<_>>(),
        ["mockingbird".into(), "pride".into()].into()
    );
}
