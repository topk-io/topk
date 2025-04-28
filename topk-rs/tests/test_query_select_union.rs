use std::collections::HashMap;
use test_context::test_context;
use topk_protos::{doc, v1::data::Value};

mod utils;
use topk_rs::query::{field, select};
use utils::ProjectTestContext;

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_select_union(ctx: &mut ProjectTestContext) {
    // create collection
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default())
        .await
        .expect("could not create collection");

    // upsert documents
    let lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "0", "rank" => 0, "mixed" => Value::null()),
            doc!("_id" => "1", "rank" => 1, "mixed" => (1 as u32)),
            doc!("_id" => "2", "rank" => 2, "mixed" => (2 as u64)),
            doc!("_id" => "3", "rank" => 3, "mixed" => (3 as i32)),
            doc!("_id" => "4", "rank" => 4, "mixed" => (4 as i64)),
            doc!("_id" => "5", "rank" => 5, "mixed" => (5 as f32)),
            doc!("_id" => "6", "rank" => 6, "mixed" => (6 as f64)),
            doc!("_id" => "7", "rank" => 7, "mixed" => true),
            doc!("_id" => "8", "rank" => 8, "mixed" => "hello"),
            doc!("_id" => "9", "rank" => 9, "mixed" => Value::byte_vector(vec![1, 2, 3])),
            doc!("_id" => "10", "rank" => 10, "mixed" => Value::float_vector(vec![1.0, 2.0, 3.0])),
            doc!("_id" => "11", "rank" => 11, "mixed" => Value::bytes(vec![1, 2, 3])),
        ])
        .await
        .expect("upsert failed");

    // wait for writes to be flushed
    let _ = ctx
        .client
        .collection(&collection.name)
        .count(Some(lsn), None)
        .await
        .expect("could not query");

    let results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("mixed", field("mixed"))]).topk(field("rank"), 100, true),
            None,
            None,
        )
        .await
        .expect("could not query");

    assert_eq!(
        results,
        vec![
            doc!("_id" => "0", "mixed" => Value::null()),
            doc!("_id" => "1", "mixed" => (1 as u32)),
            doc!("_id" => "2", "mixed" => (2 as u64)),
            doc!("_id" => "3", "mixed" => (3 as i32)),
            doc!("_id" => "4", "mixed" => (4 as i64)),
            doc!("_id" => "5", "mixed" => (5.0 as f32)),
            doc!("_id" => "6", "mixed" => (6.0 as f64)),
            doc!("_id" => "7", "mixed" => true),
            doc!("_id" => "8", "mixed" => "hello"),
            doc!("_id" => "9", "mixed" => Value::byte_vector(vec![1, 2, 3])),
            doc!("_id" => "10", "mixed" => Value::float_vector(vec![1.0, 2.0, 3.0])),
            doc!("_id" => "11", "mixed" => Value::bytes(vec![1, 2, 3])),
        ]
    );
}
