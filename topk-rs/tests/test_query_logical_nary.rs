use test_context::test_context;
use topk_rs::proto::v1::data::Value;
use topk_rs::query::{all, any, field, filter, not, select};
use topk_rs::{doc, Error};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_any_codes_vec(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(any(vec![
                field("codes").contains("DOI 10.1000/182"),
                field("codes").contains("Barcode 0618346252"),
                field("codes").contains("UPC 025192354670"),
            ]))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["1984", "lotr", "mockingbird"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_all_codes_vec(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(all(vec![
                field("tags").contains("wizard"),
                field("tags").contains("school"),
                field("tags").contains("magic"),
            ]))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_select_any_flag(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let mut results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "has_code",
                any(vec![
                    field("codes").contains("DOI 10.1000/182"),
                    field("codes").contains("OCLC 934546789"),
                ]),
            )])
            .filter(field("_id").in_(Value::list(vec![
                "1984".to_string(),
                "pride".to_string(),
                "lotr".to_string(),
            ])))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    results.sort_by(|d1, d2| d1.id().unwrap().cmp(d2.id().unwrap()));

    assert_eq!(
        results,
        vec![
            doc!("_id" => "1984", "has_code" => true),
            doc!("_id" => "lotr",  "has_code" => false),
            doc!("_id" => "pride", "has_code" => true),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_select_all_flag(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let mut results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "all_match",
                all(vec![
                    field("codes").contains("UPC 074327356709"),
                    field("codes").contains("ASIN B000FC0SIS"),
                ]),
            )])
            .filter(field("_id").in_(Value::list(vec!["gatsby".to_string(), "pride".to_string()])))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    results.sort_by(|d1, d2| d1.id().unwrap().cmp(d2.id().unwrap()));

    assert_eq!(
        results,
        vec![
            doc!("_id" => "gatsby", "all_match" => true),
            doc!("_id" => "pride",  "all_match" => false),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_nested_any_all(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let expr = any(vec![
        all(vec![
            field("tags").contains("wizard"),
            field("tags").contains("magic"),
        ]),
        all(vec![
            field("codes").contains("UPC 074327356709"),
            field("codes").contains("ASIN B000FC0SIS"),
        ]),
    ]);

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(expr).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["gatsby", "lotr", "harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_non_nested_any_and_all(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let codes_any = any(vec![
        field("codes").contains("Barcode 0618346252"),
        field("codes").contains("UPC 043970818909"),
    ]);

    let tags_all = all(vec![
        field("tags").contains("wizard"),
        field("tags").contains("magic"),
    ]);

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(codes_any.and(tags_all)).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["lotr", "harry"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_any_mixed_exprs(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(any(vec![
                field("title").starts_with("The Great"),
                field("tags").contains("romance"),
                field("published_year").lt(1900u32),
            ]))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["pride", "moby", "gatsby"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_all_mixed_exprs(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(all(vec![
                field("published_year").gt(1900u32),
                field("title").contains("The"),
                not(field("tags").contains("romance")),
            ]))
            .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["catcher", "hobbit", "lotr", "alchemist"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_all_large_arity(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let expr = all(vec![field("tags").contains("wizard"); 32]);

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(expr).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(result, ["harry", "lotr"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_all_max_arity(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let expr = all(vec![field("tags").contains("wizard"); 33]);

    let err = ctx
        .client
        .collection(&collection.name)
        .query(
            filter(expr).topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect_err("should have failed due to max arity");

    assert!(matches!(err, Error::InvalidArgument(_)));
}
