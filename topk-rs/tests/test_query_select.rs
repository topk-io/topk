use std::collections::HashMap;
use test_context::test_context;
use topk_protos::doc;
use topk_protos::v1::data::Value;

mod utils;
use topk_rs::data::Vector;
use topk_rs::query::{field, fns, literal, r#match, select};
use utils::dataset;
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_literal(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("literal", literal(1.0))])
                .filter(field("title").eq("1984"))
                .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results, vec![doc!("_id" => "1984", "literal" => 1.0)]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_non_existing_field(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("literal", field("non_existing_field"))])
                .filter(field("title").eq("1984"))
                .topk(field("published_year"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(results, vec![doc!("_id" => "1984")]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_limit(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("published_year"), 3, true),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 3);

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("published_year"), 2, true),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 2);

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))]).topk(field("published_year"), 1, true),
            None,
            None,
        )
        .await
        .expect("could not query");
    assert_eq!(results.len(), 1);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_asc(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("published_year", field("published_year"))]).topk(
                field("published_year"),
                3,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "pride", "published_year" => 1813 as u32),
            doc!("_id" => "moby", "published_year" => 1851 as u32),
            doc!("_id" => "gatsby", "published_year" => 1925 as u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_topk_desc(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("published_year", field("published_year"))]).topk(
                field("published_year"),
                3,
                false,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "harry", "published_year" => 1997 as u32),
            doc!("_id" => "alchemist", "published_year" => 1988 as u32),
            doc!("_id" => "mockingbird", "published_year" => 1960 as u32),
        ]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_bm25_score(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("bm25_score", fns::bm25_score())])
                .filter(r#match("pride", None, None))
                .topk(field("bm25_score"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![doc!("_id" => "pride", "bm25_score" => 2.0774152 as f32)]
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_vector_distance(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([(
                "summary_distance",
                fns::vector_distance("summary_embedding", Vector::F32(vec![2.0; 16])),
            )])
            .topk(field("summary_distance"), 3, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids!(results, ["1984", "mockingbird", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_null_field(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    ctx.client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "1984", "a" => Value::null()),
            doc!("_id" => "pride"),
        ])
        .await
        .expect("could not upsert documents");

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("a", field("a")), ("b", literal(1 as u32))]).topk(field("b"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    // Assert that `a` is null for all documents, even when not specified when upserting
    assert_eq!(
        results
            .into_iter()
            .map(|d| d.fields.get("a").unwrap().clone())
            .collect::<Vec<_>>(),
        vec![Value::null(), Value::null()]
    );
}
