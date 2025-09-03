use test_context::test_context;
use topk_rs::data::literal;
use topk_rs::query::{field, filter, not, r#match, select};
use topk_rs::{doc, Error};

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
            select([(
                "clamped_year",
                field("published_year").max(1890u32).min(1910u32),
            )])
            .filter(r#match(
                "millionaire love consequences dwarves",
                Some("summary"),
                None,
                false,
            ))
            .topk(field("published_year"), 3, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    for (i, (id, year)) in [("pride", 1890u32), ("moby", 1890), ("gatsby", 1910)]
        .iter()
        .enumerate()
    {
        assert_eq!(result[i].id().unwrap(), *id);
        assert_eq!(
            result[i].fields.get("clamped_year").unwrap(),
            &literal(*year)
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_logical_deep_recursion_limit(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    // Build a deeply nested OR expression that should trigger recursion limit
    // Start with a simple logical filter
    let mut deep_expr = field("published_year").eq(1990u32);

    // Add nested OR expressions to exceed router depth (16) but stay below protobuf decode limit
    for i in 0..20 {
        deep_expr = deep_expr.or(field("published_year").eq(1990u32 + i));
    }

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(deep_expr).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed due to recursion limit");

    assert!(matches!(err, Error::InvalidArgument(_) if err.to_string().contains("too deep")));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_gt_and_lte_string(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(field("_id").gt("moby").and(field("_id").lte("pride"))).topk(
                field("published_year"),
                100,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["mockingbird", "pride"]);
}


#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_min_string(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("title", field("title")),
                ("min_string", field("title").min("Oz")),
            ])
            .topk(field("published_year"), 2, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![
            doc!("_id" => "pride", "title" => "Pride and Prejudice", "min_string" => "Oz"),
            doc!("_id" => "moby", "title" => "Moby-Dick", "min_string" => "Moby-Dick"),
        ]
    );
}
