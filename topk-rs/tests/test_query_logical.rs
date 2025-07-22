use test_context::test_context;
use topk_rs::data::literal;
use topk_rs::doc;
use topk_rs::query::{field, filter, fns, not, r#match, select};

mod utils;
use utils::{dataset, ProjectTestContext};

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_lte(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("published_year").lte(1950 as u32)).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984", "pride", "hobbit", "moby", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_and(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(
                field("published_year")
                    .lte(1950 as u32)
                    .and(field("published_year").gte(1948 as u32)),
            )
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_is_null(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("nullable_embedding").is_null()).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(
        result,
        ["pride", "gatsby", "moby", "hobbit", "lotr", "alchemist"]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_is_not_null(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("nullable_embedding").is_not_null()).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["mockingbird", "1984", "catcher", "harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_not(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(not(field("_id").contains("gatsby"))).topk(field("published_year"), 100, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(
        result,
        [
            "harry",
            "lotr",
            "1984",
            "mockingbird",
            "moby",
            "alchemist",
            "catcher",
            "hobbit",
            "pride"
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_choose_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "love_score",
                field("summary").match_all("love").choose(2.0, 0.1),
            )])
            .filter(field("love_score").gt(1.0))
            .topk(field("love_score"), 10, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_choose_literal_and_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "love_score",
                field("summary")
                    .match_all("love")
                    .choose(field("published_year"), 10u32),
            )])
            .topk(field("love_score"), 2, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("_id" => "gatsby", "love_score" => 1925u32),
            doc!("_id" => "pride", "love_score" => 1813u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_choose_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "love_score",
                field("summary")
                    .match_all("love")
                    .choose(field("published_year"), field("published_year").div(10)),
            )])
            .topk(field("love_score"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("_id" => "gatsby", "love_score" => 1925u32),
            doc!("_id" => "pride", "love_score" => 1813u32),
            doc!("_id" => "harry", "love_score" => 199u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_coalesce_nullable(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("importance", field("nullable_importance").coalesce(1.0_f32))])
                .filter(field("published_year").lt(1900))
                .topk(field("published_year"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("_id" => "moby", "importance" => 5.0_f32),
            doc!("_id" => "pride", "importance" => 1.0_f32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_coalesce_missing(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("importance", field("missing_field").coalesce(1.0_f32))])
                .filter(field("published_year").lt(1900))
                .topk(field("published_year"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("_id" => "moby", "importance" => 1.0_f32),
            doc!("_id" => "pride", "importance" => 1.0_f32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_coalesce_non_nullable(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("coalesced_year", field("published_year").coalesce(0u32))])
                .filter(field("published_year").lt(1900))
                .topk(field("published_year"), 3, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("_id" => "moby", "coalesced_year" => 1851u32),
            doc!("_id" => "pride", "coalesced_year" => 1813u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_abs(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("abs_year", field("published_year").add(-1990).abs())]).topk(
                field("abs_year"),
                3,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    // The 3 books closest to 1990
    assert_eq!(
        result,
        vec![
            doc!("_id" => "alchemist", "abs_year" => 2_i64),
            doc!("_id" => "harry", "abs_year" => 7_i64),
            doc!("_id" => "mockingbird", "abs_year" => 30_i64),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_min_max(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("bm25_score", fns::bm25_score())])
                .select([("clamped_bm25_score", field("bm25_score").min(2.0).max(1.6))])
                .filter(r#match(
                    "millionaire love consequences dwarves",
                    None,
                    Some(1.0),
                    false,
                ))
                .topk(field("clamped_bm25_score"), 5, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    println!("{:?}", &result);

    assert_eq!(result.len(), 4);

    // Check document IDs and clamped scores
    assert_eq!(result[0].fields.get("_id").unwrap(), &literal("gatsby"));
    assert_eq!(
        result[0].fields.get("clamped_bm25_score").unwrap(),
        &literal(2.0_f64)
    );

    assert_eq!(result[1].fields.get("_id").unwrap(), &literal("hobbit"));
    if let Some(score_val) = result[1].fields.get("clamped_bm25_score") {
        if let Some(score) = score_val.as_f64() {
            assert!(score >= 1.6 && score <= 2.0);
        }
    }

    assert_eq!(result[2].fields.get("_id").unwrap(), &literal("moby"));
    if let Some(score_val) = result[2].fields.get("clamped_bm25_score") {
        if let Some(score) = score_val.as_f64() {
            assert!(score >= 1.6 && score <= 2.0);
        }
    }

    assert_eq!(result[3].fields.get("_id").unwrap(), &literal("pride"));
    assert_eq!(
        result[3].fields.get("clamped_bm25_score").unwrap(),
        &literal(1.6_f64)
    );
}
