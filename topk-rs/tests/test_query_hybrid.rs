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
    assert_doc_ids_ordered!(&result, ["mockingbird", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_hybrid_keyword_boost(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Multiply summary_distance by 0.1 if the summary matches "racial injustice", otherwise
    // multiply by 1.0 (leave unchanged).
    for score_expr in [
        field("summary_distance")
            * (field("summary")
                .match_all("racial injustice")
                .choose(0.1, 1.0)),
        field("summary_distance").boost(field("summary").match_all("racial injustice"), 0.1),
    ] {
        let result = ctx
            .client
            .collection(&collection.name)
            .query(
                select([(
                    "summary_distance",
                    fns::vector_distance("summary_embedding", f32_vector(vec![2.3; 16])),
                )])
                .topk(score_expr, 3, true),
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
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_hybrid_coalesce_score(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                (
                    "summary_score",
                    fns::vector_distance("summary_embedding", f32_vector(vec![4.1; 16])),
                ),
                (
                    "nullable_score",
                    fns::vector_distance("nullable_embedding", f32_vector(vec![4.1; 16])),
                ),
            ])
            .topk(
                field("summary_score") + field("nullable_score").coalesce(0.0),
                3,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    // Adding the nullable_score without coalescing would exclude "pride" and "gatsby" from
    // the result set, even though they are the closest candidates based on summary_score.
    assert_doc_ids_ordered!(&result, ["gatsby", "pride", "catcher"]);
}
