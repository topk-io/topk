use std::collections::HashSet;
use test_context::test_context;
use topk_rs::data::{f32_vector, literal};
use topk_rs::query::{field, fns, r#match, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

use crate::utils::is_sorted;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_hybrid_vector_bm25(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "summary_distance",
                    fns::vector_distance("summary_embedding", f32_vector(vec![2.0; 16])),
                ),
                ("bm25_score", fns::bm25_score()),
            ])
            .filter(r#match("love", None, Some(30.0), false).or(r#match(
                "young",
                None,
                Some(10.0),
                false,
            )))
            .topk(
                field("bm25_score") + (field("summary_distance").mul(literal(100))),
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

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_hybrid_keyword_boost(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("summary_embedding", f32_vector(vec![2.3; 16])),
            )])
            .topk(
                // Multiply summary_distance by 0.1 if the summary matches "racial injustice", otherwise
                // multiply by 1.0 (leave unchanged).
                field("summary_distance")
                    * (field("summary")
                        .match_all("racial injustice")
                        .choose(0.1, 1.0)),
                3,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    // Keyword boosting swaps the order of results so we expect [1984, mockingbird, pride]
    // instead of [1984, pride, mockingbird].
    assert_doc_ids_ordered!(&result, ["1984", "mockingbird", "pride"]);

    // We use a modified scoring expression so the results are not sorted by summary_distance.
    assert!(!is_sorted(&result, "summary_distance"));
}
