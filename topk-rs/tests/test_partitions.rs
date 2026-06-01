use std::collections::HashMap;

use test_context::test_context;

use topk_rs::doc;
use topk_rs::proto::v1::data::Value;
use topk_rs::query::{field, select};
use topk_rs::Error;

mod utils;
use utils::ProjectTestContext;
use uuid::Uuid;

macro_rules! assert_partition_metadata {
    ($docs:expr, $partition:expr) => {{
        for doc in $docs {
            assert_eq!(
                doc.fields.get("partition"),
                Some(&Value::string($partition)),
                "doc `{}` has unexpected partition metadata",
                doc.id().unwrap(),
            );
        }
    }};
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_upsert_isolation(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let default_lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "shared", "partition" => "default"),
            doc!("_id" => "only-default", "partition" => "default"),
        ])
        .await
        .expect("could not upsert to default partition");
    assert_eq!(&default_lsn, "1");

    let p1_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .upsert(vec![
            doc!("_id" => "shared", "partition" => "p1"),
            doc!("_id" => "only-p1", "partition" => "p1"),
        ])
        .await
        .expect("could not upsert to partition p1");
    assert_eq!(&p1_lsn, "1");

    let p2_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .upsert(vec![doc!("_id" => "shared", "partition" => "p2")])
        .await
        .expect("could not upsert to partition p2");
    assert_eq!(&p2_lsn, "1");

    let default_docs = ctx
        .client
        .collection(&collection.name)
        .get(
            ["shared", "only-default", "only-p1"],
            None,
            Some(default_lsn),
            None,
        )
        .await
        .expect("could not get from default partition");

    assert_eq!(
        default_docs,
        HashMap::from([
            (
                "shared".to_string(),
                doc!("_id" => "shared", "partition" => "default").fields
            ),
            (
                "only-default".to_string(),
                doc!("_id" => "only-default", "partition" => "default").fields
            ),
        ])
    );

    let p1_docs = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .get(
            ["shared", "only-default", "only-p1"],
            None,
            Some(p1_lsn),
            None,
        )
        .await
        .expect("could not get from partition p1");

    assert_eq!(
        p1_docs,
        HashMap::from([
            (
                "shared".to_string(),
                doc!("_id" => "shared", "partition" => "p1").fields
            ),
            (
                "only-p1".to_string(),
                doc!("_id" => "only-p1", "partition" => "p1").fields
            ),
        ])
    );

    let p2_docs = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .get(["shared"], None, Some(p2_lsn), None)
        .await
        .expect("could not get from partition p2");

    assert_eq!(
        p2_docs,
        HashMap::from([(
            "shared".to_string(),
            doc!("_id" => "shared", "partition" => "p2").fields
        )])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_update(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    ctx.client
        .collection(&collection.name)
        .partition("p1")
        .upsert(vec![doc!("_id" => "doc", "value" => "p1-v1")])
        .await
        .expect("could not upsert to partition p1");

    let p2_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .upsert(vec![doc!("_id" => "doc", "value" => "p2-v1")])
        .await
        .expect("could not upsert to partition p2");

    let p1_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .update(
            vec![doc!("_id" => "doc", "value" => "p1-v2", "extra" => "p1")],
            false,
        )
        .await
        .expect("could not update partition p1");
    assert_eq!(&p1_lsn, "2");

    let p1_docs = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .get(["doc"], None, Some(p1_lsn), None)
        .await
        .expect("could not get from partition p1");

    assert_eq!(
        p1_docs["doc"],
        doc!("_id" => "doc", "value" => "p1-v2", "extra" => "p1").fields
    );

    let p2_docs = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .get(["doc"], None, Some(p2_lsn), None)
        .await
        .expect("could not get from partition p2");

    assert_eq!(
        p2_docs["doc"],
        doc!("_id" => "doc", "value" => "p2-v1").fields
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_delete(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let p1 = ctx.client.collection(&collection.name).partition("p1");
    let p2 = ctx.client.collection(&collection.name).partition("p2");

    let p1_lsn = p1
        .upsert(vec![
            doc!("_id" => "doc1", "partition" => "p1", "rank" => 1),
            doc!("_id" => "doc2", "partition" => "p1", "rank" => 2),
        ])
        .await
        .expect("could not upsert to partition p1");
    assert_eq!(p1_lsn, "1");

    let p2_lsn = p2
        .upsert(vec![
            doc!("_id" => "doc1", "partition" => "p2", "rank" => 3),
        ])
        .await
        .expect("could not upsert to partition p2");
    assert_eq!(p2_lsn, "1");

    let count = p1
        .count(Some(p1_lsn), None)
        .await
        .expect("could not count partition p1");
    assert_eq!(count, 2);

    let p1_lsn = p1
        .delete(vec!["doc1".to_string()])
        .await
        .expect("could not delete from partition p1");
    assert_eq!(&p1_lsn, "2");

    let p1_docs = p1
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))]).topk(
                field("rank"),
                100,
                true,
            ),
            Some(p1_lsn),
            None,
        )
        .await
        .expect("could not query partition p1");

    assert_partition_metadata!(&p1_docs, "p1");
    assert_doc_ids!(p1_docs, ["doc2"]);

    let p2_docs = p2
        .get(["doc1"], None, Some(p2_lsn), None)
        .await
        .expect("could not get from partition p2");

    assert_eq!(
        p2_docs,
        HashMap::from([(
            "doc1".to_string(),
            doc!("_id" => "doc1", "partition" => "p2", "rank" => 3).fields
        )])
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_query_count(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let default_lsn = ctx
        .client
        .collection(&collection.name)
        .upsert(vec![
            doc!("_id" => "doc1", "partition" => "default"),
            doc!("_id" => "doc2", "partition" => "default"),
        ])
        .await
        .expect("could not upsert to default partition");
    assert_eq!(default_lsn, "1");

    let p1_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .upsert(vec![
            doc!("_id" => "doc1", "partition" => "p1"),
            doc!("_id" => "doc2", "partition" => "p1"),
            doc!("_id" => "doc3", "partition" => "p1"),
        ])
        .await
        .expect("could not upsert to partition p1");
    assert_eq!(p1_lsn, "1");

    let p2_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .upsert(vec![doc!("_id" => "doc1", "partition" => "p2")])
        .await
        .expect("could not upsert to partition p2");
    assert_eq!(p2_lsn, "1");

    let default_count = ctx
        .client
        .collection(&collection.name)
        .count(Some(default_lsn.clone()), None)
        .await
        .expect("could not count default partition");
    assert_eq!(default_count, 2);

    let p1_count = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .count(Some(p1_lsn.clone()), None)
        .await
        .expect("could not count partition p1");
    assert_eq!(p1_count, 3);

    let p2_count = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .count(Some(p2_lsn.clone()), None)
        .await
        .expect("could not count partition p2");
    assert_eq!(p2_count, 1);

    let default_docs = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))]).limit(10),
            Some(default_lsn),
            None,
        )
        .await
        .expect("could not query default partition");

    assert_partition_metadata!(&default_docs, "default");
    assert_doc_ids!(default_docs, ["doc1", "doc2"]);

    let p1_docs = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))]).limit(10),
            Some(p1_lsn),
            None,
        )
        .await
        .expect("could not query partition p1");

    assert_partition_metadata!(&p1_docs, "p1");
    assert_doc_ids!(p1_docs, ["doc1", "doc2", "doc3"]);

    let p2_docs = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))]).limit(10),
            Some(p2_lsn),
            None,
        )
        .await
        .expect("could not query partition p2");

    assert_partition_metadata!(&p2_docs, "p2");
    assert_doc_ids!(p2_docs, ["doc1"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_query_filter(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let p1_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .upsert(vec![
            doc!("_id" => "doc1", "partition" => "p1", "region" => "us"),
            doc!("_id" => "doc2", "partition" => "p1", "region" => "eu"),
            doc!("_id" => "doc3", "partition" => "p1", "region" => "us"),
        ])
        .await
        .expect("could not upsert to partition p1");

    assert_eq!(p1_lsn, "1");

    let p2_lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .upsert(vec![
            doc!("_id" => "doc1", "partition" => "p2", "region" => "us"),
            doc!("_id" => "doc2", "partition" => "p2", "region" => "us"),
        ])
        .await
        .expect("could not upsert to partition p2");

    assert_eq!(p2_lsn, "1");

    let p1_results = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))])
                .filter(field("region").eq("us"))
                .limit(10),
            Some(p1_lsn),
            None,
        )
        .await
        .expect("could not query partition p1");

    assert_partition_metadata!(&p1_results, "p1");
    assert_doc_ids!(p1_results, ["doc1", "doc3"]);

    let p2_results = ctx
        .client
        .collection(&collection.name)
        .partition("p2")
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))])
                .filter(field("region").eq("us"))
                .limit(10),
            Some(p2_lsn),
            None,
        )
        .await
        .expect("could not query partition p2");

    assert_partition_metadata!(&p2_results, "p2");
    assert_doc_ids!(p2_results, ["doc1", "doc2"]);

    // Query for default partition should return no results
    let default_results = ctx
        .client
        .collection(&collection.name)
        .query(
            select([("_id", field("_id")), ("partition", field("partition"))])
                .filter(field("region").eq("us"))
                .limit(10),
            None,
            None,
        )
        .await
        .expect("could not query partition p2");

    assert!(default_results.is_empty());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_get(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    // Write docs to partition p1
    let lsn = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .upsert(vec![
            doc!("_id" => "one", "title" => "first"),
            doc!("_id" => "two", "title" => "second"),
        ])
        .await
        .expect("could not upsert to partition p1");

    // Get docs from partition p1
    let docs = ctx
        .client
        .collection(&collection.name)
        .partition("p1")
        .get(["one", "two", "missing"], None, Some(lsn.clone()), None)
        .await
        .expect("could not get from partition p1");

    assert_eq!(
        docs,
        HashMap::from([
            (
                "one".to_string(),
                doc!("_id" => "one", "title" => "first").fields
            ),
            (
                "two".to_string(),
                doc!("_id" => "two", "title" => "second").fields
            ),
        ])
    );

    // Get docs from default partition
    let default_docs = ctx
        .client
        .collection(&collection.name)
        .get(["one", "two"], None, None, None)
        .await
        .expect("could not get from default partition");

    assert_eq!(default_docs, HashMap::new());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_query_non_existent_partition(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(&collection.name)
        .partition("missing-partition")
        .count(None, None)
        .await
        .expect_err("should not be able to query a partition that was never created");

    assert!(matches!(err, Error::CollectionNotFound));

    let err = ctx
        .client
        .collection(&collection.name)
        .partition("missing-partition")
        .get(["doc"], None, None, None)
        .await
        .expect_err("should not be able to get from a partition that was never created");

    assert!(matches!(err, Error::CollectionNotFound));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_upsert_creates_partition(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let lsn = ctx
        .client
        .collection(&collection.name)
        .partition("new-partition")
        .upsert(vec![doc!("_id" => "one", "value" => "created")])
        .await
        .expect("could not upsert to new partition");
    assert_eq!(&lsn, "1");

    let docs = ctx
        .client
        .collection(&collection.name)
        .partition("new-partition")
        .get(["one"], None, Some(lsn.clone()), None)
        .await
        .expect("could not get from newly created partition");

    assert_eq!(
        docs,
        HashMap::from([(
            "one".to_string(),
            doc!("_id" => "one", "value" => "created").fields
        )])
    );

    let count = ctx
        .client
        .collection(&collection.name)
        .partition("new-partition")
        .count(Some(lsn), None)
        .await
        .expect("could not count newly created partition");

    assert_eq!(count, 1);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_with_invalid_name(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    let err = ctx
        .client
        .collection(&collection.name)
        .partition("$foo&bar")
        .upsert(vec![doc!("_id" => "one", "value" => "created")])
        .await
        .expect_err("expected invalid partition name error");

    assert!(err.to_string().contains("invalid partition name"));
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partition_valid_names(ctx: &mut ProjectTestContext) {
    let collection = ctx
        .client
        .collections()
        .create(ctx.wrap("test"), HashMap::default(), None)
        .await
        .expect("could not create collection");

    for name in [
        "valid_name",
        "valid-name",
        "valid_name_123",
        "valid-name-123",
        "1234",
        Uuid::new_v4().to_string().as_str(),
    ] {
        let lsn = ctx
            .client
            .collection(&collection.name)
            .partition(name)
            .upsert(vec![doc!("_id" => "one", "value" => "created")])
            .await
            .expect("could not upsert to partition");

        assert_eq!(lsn, "1");
    }
}
