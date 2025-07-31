use test_context::test_context;
use topk_rs::doc;
use topk_rs::query::{field, fns, r#match, select};

mod utils;
use utils::dataset;
use utils::ProjectTestContext;

use assert_approx_eq::assert_approx_eq;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_exp_ln(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("bm25_score", fns::bm25_score())])
                .select([
                    ("bm25_score_scale", (field("bm25_score").mul(1.5)).exp()),
                    ("bm25_score_smooth", (field("bm25_score").add(1)).ln()),
                ])
                .filter(r#match(
                    "millionaire love consequences dwarves",
                    Some("summary"),
                    Some(1.0),
                    false,
                ))
                .topk(field("bm25_score_scale"), 2, false),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids_ordered!(&result, ["gatsby", "hobbit"]);

    for doc in &result {
        let bm25_score = doc.fields.get("bm25_score").unwrap().as_f32().unwrap();
        let bm25_score_scale = doc
            .fields
            .get("bm25_score_scale")
            .unwrap()
            .as_f32()
            .unwrap();
        let bm25_score_smooth = doc
            .fields
            .get("bm25_score_smooth")
            .unwrap()
            .as_f32()
            .unwrap();
        assert_approx_eq!(bm25_score_scale, (bm25_score * 1.5).exp(), 1e-4);
        assert_approx_eq!(bm25_score_smooth, (bm25_score + 1.0).ln(), 1e-4);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_float_inf(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("to_infinity", field("published_year").exp())]).topk(
                field("published_year"),
                2,
                true,
            ),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(result.len(), 2);

    for doc in &result {
        let to_infinity = doc.fields.get("to_infinity").unwrap().as_f32().unwrap();
        assert_eq!(to_infinity, f32::INFINITY);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sqrt_square(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([
                ("published_year", field("published_year")),
                ("published_year_2", field("published_year").sqrt().square()),
            ])
            .topk(field("published_year_2"), 2, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_doc_ids_ordered!(&result, ["pride", "moby"]);

    for doc in &result {
        let year_2 = doc
            .fields
            .get("published_year_2")
            .unwrap()
            .as_f32()
            .unwrap();
        let year_orig = doc.fields.get("published_year").unwrap().as_u32().unwrap();
        assert_eq!(year_2.round() as u32, year_orig);
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_sqrt_filter(ctx: &mut ProjectTestContext) {
    let collection = dataset::books::setup(ctx).await;

    let result = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("title", field("title"))])
                .filter(field("published_year").sqrt().gt(1990_f32.sqrt()))
                .topk(field("published_year"), 2, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        result,
        vec![doc!("_id" => "harry", "title" => "Harry Potter and the Sorcerer's Stone"),]
    );
}
