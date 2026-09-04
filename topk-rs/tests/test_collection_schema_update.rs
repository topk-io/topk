use std::collections::HashMap;
use std::time::{Duration, Instant};

use test_context::test_context;
use topk_rs::{
    doc,
    proto::v1::{
        control::{
            field_type_list::ListValueType, Collection, FieldIndex, FieldSpec, KeywordIndexType,
            VectorDistanceMetric,
        },
        data::{stage::sort_stage::SortOrder, Document, SparseVector, Value},
    },
    query::{field, filter, fns, r#match, select},
    schema, Error,
};

mod utils;
use utils::ProjectTestContext;

fn keyword(spec: FieldSpec) -> FieldSpec {
    spec.with_index(FieldIndex::keyword(KeywordIndexType::Text))
}

async fn create(ctx: &mut ProjectTestContext, schema: HashMap<String, FieldSpec>) -> Collection {
    create_named(ctx, "books", schema).await
}

async fn create_named(
    ctx: &mut ProjectTestContext,
    name: &str,
    schema: HashMap<String, FieldSpec>,
) -> Collection {
    ctx.client
        .collections()
        .create(ctx.wrap(name), schema, None)
        .await
        .expect("could not create collection")
}

fn docs() -> Vec<Document> {
    vec![
        doc!("_id" => "pride", "title" => "Pride and Prejudice", "summary" => "a love story in georgian england", "rating" => 5u32),
        doc!("_id" => "gatsby", "title" => "The Great Gatsby", "summary" => "love and loss in the jazz age", "rating" => 4u32),
        doc!("_id" => "moby", "title" => "Moby Dick", "summary" => "a whale and a captain", "rating" => 3u32),
    ]
}

async fn update(
    ctx: &ProjectTestContext,
    name: &str,
    schema: HashMap<String, FieldSpec>,
    drop_fields: &[&str],
) -> Result<Collection, Error> {
    ctx.client
        .collections()
        .update(
            name,
            schema,
            drop_fields.iter().map(|f| f.to_string()).collect(),
        )
        .await
}

async fn upsert(ctx: &ProjectTestContext, name: &str, docs: Vec<Document>) -> String {
    ctx.client
        .collection(name)
        .upsert(docs)
        .await
        .expect("could not upsert")
}

async fn ids_matching(
    ctx: &ProjectTestContext,
    collection: &str,
    term: &str,
    fieldname: &str,
    lsn: Option<String>,
) -> Result<Vec<String>, Error> {
    let docs = ctx
        .client
        .collection(collection)
        .query(
            filter(r#match(term, Some(fieldname), None, false))
                .select([("title", field("title"))])
                .limit(100),
            lsn,
            None,
        )
        .await?;
    let mut ids: Vec<String> = docs.iter().map(|d| d.id().unwrap().to_string()).collect();
    ids.sort();
    Ok(ids)
}

/// Polls `probe` until it succeeds. A query on an index still being built fails with the
/// index-missing code until the compactor has rewritten every file holding the field.
async fn wait_for_index<T>(probe: impl AsyncFn() -> Result<T, Error>) -> T {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match probe().await {
            Ok(value) => return value,
            Err(Error::Unexpected(msg)) if msg.contains("Missing index") => {}
            Err(Error::InvalidArgument(msg)) if msg.contains("Missing keyword index") => {}
            Err(err) => panic!("query on a building index: {err:?}"),
        }
        assert!(Instant::now() < deadline, "index still building after 120s");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn reason(err: Error) -> String {
    match err {
        Error::Unexpected(msg) => msg,
        e => panic!("{e:?}"),
    }
}

/// A keyword index added to a field that already has data is BUILDING until the compactor has
/// rewritten every file holding the field, then queries on it work.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_add_index_on_written_field(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => keyword(FieldSpec::text(true)))).await;
    let name = collection.name.clone();
    ctx.client.collection(&name).upsert(docs()).await.unwrap();

    // `summary` is stored but undeclared: no keyword index to match on
    ids_matching(ctx, &name, "love", "summary", None)
        .await
        .expect_err("undeclared field has no index");

    let updated = ctx
        .client
        .collections()
        .update(
            &name,
            schema!("summary" => keyword(FieldSpec::text(false))),
            vec![],
        )
        .await
        .expect("update failed");
    assert!(updated.schema.contains_key("summary"));

    // the index builds in the background; searches fail with a clear error until it is ready
    let ids =
        wait_for_index(async || ids_matching(ctx, &name, "love", "summary", None).await).await;
    assert_eq!(ids, vec!["gatsby", "pride"]);

    // filters on the field kept working throughout, and new writes are indexed directly
    let lsn = ctx
        .client
        .collection(&name)
        .upsert(vec![
            doc!("_id" => "new", "title" => "New", "summary" => "love again"),
        ])
        .await
        .unwrap();
    let ids = ids_matching(ctx, &name, "love", "summary", Some(lsn))
        .await
        .unwrap();
    assert_eq!(ids, vec!["gatsby", "new", "pride"]);
}

/// `required` is accepted when every stored document has the field and rejected, naming the
/// field, when one lacks it. A rejected update leaves the schema untouched.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_required_is_proven_against_data(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => FieldSpec::text(true))).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;

    let updated = update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .expect("every document has a rating");
    assert!(updated.schema["rating"].required);

    let err = ctx
        .client
        .collection(&name)
        .upsert(vec![doc!("_id" => "x", "title" => "no rating")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::DocumentValidationError(_)), "{err:?}");

    let err = update(ctx, &name, schema!("isbn" => FieldSpec::text(true)), &[])
        .await
        .unwrap_err();
    assert!(reason(err).contains("isbn"));
    let fetched = ctx.client.collections().get(&name).await.unwrap();
    assert_eq!(fetched.schema, updated.schema);

    // a type the data contradicts is rejected too
    let err = update(
        ctx,
        &name,
        schema!("summary" => FieldSpec::integer(false)),
        &[],
    )
    .await
    .unwrap_err();
    assert!(reason(err).contains("summary"));
}

/// Dropping an index is immediate; dropping a field keeps its data filterable.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_drop_index_and_field(ctx: &mut ProjectTestContext) {
    let collection = create(
        ctx,
        schema!(
            "title" => keyword(FieldSpec::text(true)),
            "summary" => keyword(FieldSpec::text(false)),
        ),
    )
    .await;
    let name = collection.name.clone();
    let lsn = ctx.client.collection(&name).upsert(docs()).await.unwrap();
    assert_eq!(
        ids_matching(ctx, &name, "love", "summary", Some(lsn))
            .await
            .unwrap(),
        vec!["gatsby", "pride"]
    );

    let updated = ctx
        .client
        .collections()
        .update(&name, schema!("summary" => FieldSpec::text(false)), vec![])
        .await
        .unwrap();
    assert!(updated.schema["summary"].index.is_none());
    // a router may keep serving the dropped index until its collection cache expires
    let deadline = Instant::now() + Duration::from_secs(120);
    while ids_matching(ctx, &name, "love", "summary", None)
        .await
        .is_ok()
    {
        assert!(
            Instant::now() < deadline,
            "dropped index still served after 120s"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let updated = update(ctx, &name, HashMap::new(), &["summary"])
        .await
        .unwrap();
    assert!(!updated.schema.contains_key("summary"));

    let docs = ctx
        .client
        .collection(&name)
        .query(
            filter(field("summary").eq("a whale and a captain")).limit(10),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].id().unwrap(), "moby");
}

async fn nearest(ctx: &ProjectTestContext, collection: &str) -> Result<Vec<String>, Error> {
    let docs = ctx
        .client
        .collection(collection)
        .query(
            select([("dist", fns::vector_distance("vector", vec![1.0f32, 0.0]))])
                .sort([(field("dist"), SortOrder::Asc)])
                .limit(2),
            None,
            None,
        )
        .await?;
    Ok(docs.iter().map(|d| d.id().unwrap().to_string()).collect())
}

/// A vector index added to a field that already holds vectors is BUILDING until the compactor
/// has rebuilt every file from its raw documents, then vector search works.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_add_vector_index_on_stored_vectors(ctx: &mut ProjectTestContext) {
    let collection = create(
        ctx,
        schema!("title" => FieldSpec::text(true), "vector" => FieldSpec::f32_vector(2, false)),
    )
    .await;
    let name = collection.name.clone();
    ctx.client
        .collection(&name)
        .upsert(vec![
            doc!("_id" => "a", "title" => "A", "vector" => vec![1.0f32, 0.0]),
            doc!("_id" => "b", "title" => "B", "vector" => vec![0.0f32, 1.0]),
            doc!("_id" => "c", "title" => "C", "vector" => vec![0.7f32, 0.7]),
        ])
        .await
        .unwrap();
    nearest(ctx, &name).await.expect_err("no vector index yet");

    let updated = ctx
        .client
        .collections()
        .update(
            &name,
            schema!("vector" => FieldSpec::f32_vector(2, false)
                .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean))),
            vec![],
        )
        .await
        .expect("update failed");
    assert!(updated.schema["vector"].index.is_some());

    assert_eq!(
        wait_for_index(async || nearest(ctx, &name).await).await,
        vec!["a", "c"]
    );
}

/// Files written before the index existed and holding no value for the field must still serve
/// the query once READY: they get the index columns in the rewrite.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_add_index_on_partially_populated_field(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => FieldSpec::text(true))).await;
    let name = collection.name.clone();
    ctx.client
        .collection(&name)
        .upsert(vec![
            doc!("_id" => "with", "title" => "A", "summary" => "a love story"),
            doc!("_id" => "without", "title" => "B"),
        ])
        .await
        .unwrap();

    ctx.client
        .collections()
        .update(
            &name,
            schema!("summary" => keyword(FieldSpec::text(false))),
            vec![],
        )
        .await
        .expect("update failed");
    assert_eq!(
        wait_for_index(async || ids_matching(ctx, &name, "love", "summary", None).await).await,
        vec!["with"]
    );
}

/// Widening never touches the data: dropping `required` lets a doc without the field in, and
/// dropping the field keeps it queryable. Re-declaring it with another type is refused by the data.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_widen_then_redeclare(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => FieldSpec::text(true))).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;

    let relaxed = update(ctx, &name, schema!("title" => FieldSpec::text(false)), &[])
        .await
        .unwrap();
    assert!(!relaxed.schema["title"].required);
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "untitled", "rating" => 1u32)],
    )
    .await;

    let dropped = update(ctx, &name, HashMap::new(), &["title"])
        .await
        .unwrap();
    assert!(!dropped.schema.contains_key("title"));

    let err = update(
        ctx,
        &name,
        schema!("title" => FieldSpec::integer(false)),
        &[],
    )
    .await
    .unwrap_err();
    assert!(reason(err).contains("title"));
}

/// A nested field cannot be declared under a name that already holds scalar values.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_nested_field_under_scalar_parent_is_rejected(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "a", "meta" => "just a string")],
    )
    .await;

    let err = update(
        ctx,
        &name,
        schema!("meta.tag" => FieldSpec::text(false)),
        &[],
    )
    .await
    .unwrap_err();
    assert!(reason(err).contains("meta"));
}

/// Validation fans out over every partition, and it reads stored segments, not live rows: a doc
/// that once lacked the field keeps blocking `required` until compaction rewrites its segment.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_required_is_checked_in_every_partition(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "a", "rating" => 5u32)]).await;
    ctx.client
        .collection(&name)
        .partition("p1")
        .upsert(vec![doc!("_id" => "b", "rating" => 4u32)])
        .await
        .unwrap();
    ctx.client
        .collection(&name)
        .partition("p2")
        .upsert(vec![doc!("_id" => "c")])
        .await
        .unwrap();

    let err = update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .unwrap_err();
    assert!(reason(err).contains("field `rating`: null or missing in some rows"));

    // overwriting `c` does not help: its old segment still holds a row without the field
    ctx.client
        .collection(&name)
        .partition("p2")
        .upsert(vec![doc!("_id" => "c", "rating" => 3u32)])
        .await
        .unwrap();
    let err = update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .unwrap_err();
    assert!(reason(err).contains("field `rating`: null or missing in some rows"));

    // a collection whose every partition always had the field accepts it
    let clean = create_named(ctx, "clean", HashMap::new()).await;
    for partition in ["p1", "p2"] {
        ctx.client
            .collection(&clean.name)
            .partition(partition)
            .upsert(vec![doc!("_id" => partition, "rating" => 1u32)])
            .await
            .unwrap();
    }
    let updated = update(
        ctx,
        &clean.name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .expect("every partition has a rating");
    assert!(updated.schema["rating"].required);
}

/// A declared type is checked against the type of every stored value. A schema-less list of
/// numbers is stored as a list, so it never satisfies a vector declaration.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_type_narrowing_follows_stored_values(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![doc!(
            "_id" => "a",
            "score" => 4.5f64,
            "count" => 3u32,
            "nums" => vec![1.0f32, 2.0, 3.0],
            "sparse" => SparseVector::new(vec![1, 5], vec![0.5f32, 0.25f32]),
        )],
    )
    .await;

    for (schema, expected) in [
        (
            schema!("score" => FieldSpec::integer(false)),
            "field `score`: stored values are Primitive(F64)",
        ),
        (
            schema!("count" => FieldSpec::float(false)),
            "field `count`: stored values are Primitive(U32)",
        ),
        (
            schema!("nums" => FieldSpec::f32_vector(3, false)),
            "field `nums`: stored values are List(",
        ),
        (
            schema!("sparse" => FieldSpec::u8_sparse_vector(false)),
            "field `sparse`: stored values are SparseMatrix(F32)",
        ),
    ] {
        let msg = reason(update(ctx, &name, schema, &[]).await.unwrap_err());
        assert!(msg.contains(expected), "expected {expected:?} in {msg:?}");
    }

    let updated = update(
        ctx,
        &name,
        schema!(
            "score" => FieldSpec::float(true),
            "count" => FieldSpec::integer(true),
            "nums" => FieldSpec::list(true, ListValueType::Float),
            "sparse" => FieldSpec::f32_sparse_vector(true),
        ),
        &[],
    )
    .await
    .expect("stored values fit the declared types");
    assert_eq!(updated.schema.len(), 4);
}

/// Nested fields are validated on their flattened column, and enforced on later writes.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_nested_required(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![
            doc!("_id" => "a", "meta" => Value::r#struct([("tag", "x".into())])),
            doc!("_id" => "b", "meta" => Value::r#struct([("tag", "y".into())])),
        ],
    )
    .await;

    update(
        ctx,
        &name,
        schema!("meta.tag" => FieldSpec::text(true)),
        &[],
    )
    .await
    .expect("every doc has meta.tag");

    let err = ctx
        .client
        .collection(&name)
        .upsert(vec![
            doc!("_id" => "c", "meta" => Value::r#struct([("other", "z".into())])),
        ])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::DocumentValidationError(_)), "{err:?}");
}

/// Writes racing an update either land under the new schema or are retried by the client until
/// they do; none is lost and none slips past the new constraint.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_writes_during_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "seed", "rating" => 0u32)]).await;

    let writer = ctx.client.clone();
    let writes = async {
        let mut lsn = None;
        for i in 0..30u32 {
            lsn = Some(
                writer
                    .collection(&name)
                    .upsert(vec![doc!("_id" => i.to_string(), "rating" => i)])
                    .await
                    .unwrap_or_else(|e| panic!("write {i}: {e:?}")),
            );
        }
        lsn
    };
    let collections = ctx.client.collections();
    let update = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let (lsn, updated) = tokio::join!(writes, update);
    assert!(updated.expect("all docs have a rating").schema["rating"].required);

    let ids: Vec<String> = (0..30).map(|i| i.to_string()).collect();
    let docs = ctx
        .client
        .collection(&name)
        .get(
            ids.iter().map(String::as_str).collect::<Vec<_>>(),
            None,
            lsn,
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 30);

    let err = ctx
        .client
        .collection(&name)
        .upsert(vec![doc!("_id" => "late")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::DocumentValidationError(_)), "{err:?}");
}

/// Two updates racing each other never both win a CAS; the loser is told to retry.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_concurrent_updates_serialize(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "a", "rating" => 1u32, "title" => "t")],
    )
    .await;

    let (one, two) = (ctx.client.collections(), ctx.client.collections());
    let (a, b) = tokio::join!(
        one.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]),
        two.update(&name, schema!("title" => FieldSpec::text(true)), vec![]),
    );
    let fetched = ctx.client.collections().get(&name).await.unwrap();
    let mut wins = 0;
    for (result, field) in [(a, "rating"), (b, "title")] {
        match result {
            Ok(_) => {
                wins += 1;
                assert!(
                    fetched.schema[field].required,
                    "{field} won but is not required"
                );
            }
            Err(err) => {
                let msg = reason(err);
                assert!(
                    msg.contains("schema update in progress")
                        || msg.contains("concurrent schema update"),
                    "{msg}"
                );
            }
        }
    }
    assert!(wins >= 1);
}

/// Validation fans out concurrently over many partitions.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_many_partitions(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    futures::future::join_all((0..25u32).map(|p| {
        let client = ctx.client.clone();
        let name = name.clone();
        async move {
            client
                .collection(&name)
                .partition(&format!("p{p}"))
                .upsert(vec![doc!("_id" => "a", "rating" => p)])
                .await
                .unwrap_or_else(|e| panic!("p{p}: {e:?}"));
        }
    }))
    .await;
    let updated = update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .expect("every partition has a rating");
    assert!(updated.schema["rating"].required);
}

/// Reads keep working over a WAL that holds barrier segments.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_reads_after_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;
    for _ in 0..3 {
        update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(true)),
            &[],
        )
        .await
        .unwrap();
        update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(false)),
            &[],
        )
        .await
        .unwrap();
    }
    let lsn = upsert(
        ctx,
        &name,
        vec![doc!("_id" => "late", "title" => "Late", "summary" => "after", "rating" => 1u32)],
    )
    .await;

    let got = ctx
        .client
        .collection(&name)
        .get(["pride", "late"], None, Some(lsn.clone()), None)
        .await
        .unwrap();
    assert_eq!(got.len(), 2);
    let found = ctx
        .client
        .collection(&name)
        .query(filter(field("rating").gte(3u32)).count(), Some(lsn), None)
        .await
        .unwrap();
    assert_eq!(found, vec![doc!("_count" => 3u64)]);
}

/// Updating a deleted collection is a not-found, not an internal error.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_deleted_collection(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    ctx.client
        .collections()
        .delete(&collection.name)
        .await
        .unwrap();
    let err = update(
        ctx,
        &collection.name,
        schema!("rating" => FieldSpec::integer(false)),
        &[],
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::CollectionNotFound), "{err:?}");
}

/// Compaction must digest barrier segments. Slow: waits for the emulator's flush interval.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "waits ~90s for the compactor; run explicitly"]
async fn test_compaction_after_barriers(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;
    update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .unwrap();
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "late", "title" => "Late", "summary" => "after", "rating" => 1u32)],
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_secs(90)).await;
    let found = ctx
        .client
        .collection(&name)
        .query(filter(field("rating").gte(1u32)).count(), None, None)
        .await
        .unwrap();
    assert_eq!(found, vec![doc!("_count" => 4u64)]);
}

/// Validation also reads compacted data files. Slow: writes enough segments to trigger a flush.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "waits for the compactor; run explicitly"]
async fn test_validation_reads_compacted_files(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    for i in 0..55u32 {
        upsert(
            ctx,
            &name,
            vec![doc!("_id" => i.to_string(), "rating" => i)],
        )
        .await;
    }
    upsert(ctx, &name, vec![doc!("_id" => "unrated")]).await;
    tokio::time::sleep(std::time::Duration::from_secs(45)).await;

    let msg = reason(
        update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(true)),
            &[],
        )
        .await
        .unwrap_err(),
    );
    assert!(
        msg.contains("field `rating`: null or missing in some rows"),
        "{msg}"
    );
    let msg = reason(
        update(ctx, &name, schema!("rating" => FieldSpec::text(false)), &[])
            .await
            .unwrap_err(),
    );
    assert!(
        msg.contains("field `rating`: stored values are Primitive(U32)"),
        "{msg}"
    );
    update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(false)),
        &[],
    )
    .await
    .expect("compacted values are integers");
}

/// Several writers hammering one partition while it is being validated: the update either
/// commits or reports a retryable conflict, never an internal error, and no write is lost.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_concurrent_writers_during_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "seed", "rating" => 0u32)]).await;

    let writers = futures::future::join_all((0..5u32).map(|w| {
        let client = ctx.client.clone();
        let name = name.clone();
        async move {
            for i in 0..10u32 {
                client
                    .collection(&name)
                    .upsert(vec![doc!("_id" => format!("{w}-{i}"), "rating" => i)])
                    .await
                    .unwrap_or_else(|e| panic!("writer {w} write {i}: {e:?}"));
            }
        }
    }));
    let collections = ctx.client.collections();
    let update = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let (_, updated) = tokio::join!(writers, update);
    match updated {
        Ok(collection) => assert!(collection.schema["rating"].required),
        Err(err) => assert!(!matches!(err, Error::Internal(_)), "{err:?}"),
    }

    let ids: Vec<String> = (0..5)
        .flat_map(|w| (0..10).map(move |i| format!("{w}-{i}")))
        .collect();
    let docs = ctx
        .client
        .collection(&name)
        .get(
            ids.iter().map(String::as_str).collect::<Vec<_>>(),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 50);
}

/// Partial updates and deletes keep working after a narrowing: the merged document is what
/// gets validated, so a partial update without the required field is fine.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partial_update_and_delete_after_narrowing(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;
    update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .unwrap();

    ctx.client
        .collection(&name)
        .update(
            vec![doc!("_id" => "pride", "title" => "Pride & Prejudice")],
            false,
        )
        .await
        .expect("partial update keeps the stored rating");
    let lsn = ctx
        .client
        .collection(&name)
        .delete(vec!["gatsby".to_string()])
        .await
        .unwrap();

    let got = ctx
        .client
        .collection(&name)
        .get(["pride", "gatsby"], None, Some(lsn), None)
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got["pride"]["title"], "Pride & Prejudice".into());
}

/// Requests the planner refuses before touching any data.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_rejected_requests(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => FieldSpec::text(true))).await;
    let name = collection.name.clone();

    let err = update(ctx, &name, schema!("_id" => FieldSpec::text(false)), &[])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::SchemaValidationError(_)), "{err:?}");

    for (schema, drop, expected) in [
        (
            schema!("title" => FieldSpec::text(false), "rating" => FieldSpec::integer(false)),
            &[][..],
            "cannot narrow and widen in one update",
        ),
        (schema!(), &["isbn"][..], "field `isbn`: unknown"),
        (
            schema!("title" => FieldSpec::text(true)),
            &["title"][..],
            "field `title`: updated and dropped",
        ),
    ] {
        let msg = reason(update(ctx, &name, schema, drop).await.unwrap_err());
        assert!(msg.contains(expected), "expected {expected:?} in {msg:?}");
    }
    assert_eq!(
        ctx.client.collections().get(&name).await.unwrap().schema,
        collection.schema
    );
}

/// A stranded pending update (its writer died) blocks other updates until its lease expires, then
/// the next caller rolls it back and proceeds. Emulator only: plants the record in dynamodb-local.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "writes to the emulator's dynamodb-local; run explicitly"]
async fn test_recovers_an_expired_pending_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;

    let key = format!(
        r#"{{"project":{{"S":"{}#{}"}},"name":{{"S":"{name}"}}}}"#,
        collection.org_id, collection.project_id
    );
    let dynamodb = |args: &[&str]| {
        std::process::Command::new("aws")
            .env("AWS_ACCESS_KEY_ID", "test")
            .env("AWS_SECRET_ACCESS_KEY", "test")
            .env("AWS_DEFAULT_REGION", "us-east-1")
            .args(["--endpoint-url", "http://localhost:8000", "dynamodb"])
            .args(args)
            .args(["--table-name", "collections", "--key", &key])
            .output()
            .expect("aws cli")
    };
    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 8;
    let pending = format!(
        r#"{{":p":{{"M":{{"schema":{{"M":{{"schema":{{"M":{{}}}}}}}},"version":{{"N":"2"}},"expires_at":{{"N":"{expires_at}"}}}}}}}}"#
    );
    let planted = dynamodb(&[
        "update-item",
        "--update-expression",
        "SET pending = :p",
        "--expression-attribute-values",
        &pending,
    ]);
    assert!(
        planted.status.success(),
        "{}",
        String::from_utf8_lossy(&planted.stderr)
    );

    let msg = reason(
        update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(true)),
            &[],
        )
        .await
        .unwrap_err(),
    );
    assert!(msg.contains("schema update in progress"), "{msg}");
    // while pending, readers see the committed schema, not the one under validation
    let visible = ctx.client.collections().get(&name).await.unwrap();
    assert_eq!(visible.schema, collection.schema);

    tokio::time::sleep(std::time::Duration::from_secs(9)).await;
    let updated = update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .expect("expired pending is rolled back, then the update proceeds");
    assert!(updated.schema["rating"].required);

    // 1 → the abandoned pending was version 2 → rollback lands on 3 → our update is 4
    let item = dynamodb(&[
        "get-item",
        "--query",
        "[Item.schema_version.N, Item.pending]",
        "--output",
        "text",
    ]);
    assert_eq!(String::from_utf8_lossy(&item.stdout).trim(), "4\tNone");
}

/// Writes racing an update that ends up rejected: while it is pending, new shards enforce the
/// pending schema, so a write valid under the current schema may be refused with a validation
/// error the client will not retry. Nothing else may go wrong, and the rollback must be clean.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_writes_during_a_rejected_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "seed")]).await;

    let writer = ctx.client.clone();
    let writes = async {
        let mut refused = 0;
        for i in 0..30u32 {
            match writer
                .collection(&name)
                .upsert(vec![doc!("_id" => i.to_string())])
                .await
            {
                Ok(_) => {}
                Err(Error::DocumentValidationError(_)) => refused += 1,
                Err(e) => panic!("write {i}: {e:?}"),
            }
        }
        refused
    };
    let collections = ctx.client.collections();
    let update = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let (refused, rejected) = tokio::join!(writes, update);
    assert!(reason(rejected.unwrap_err()).contains("field `rating`: null or missing in some rows"));

    let fetched = ctx.client.collections().get(&name).await.unwrap();
    assert!(fetched.schema.is_empty(), "rolled back");
    upsert(ctx, &name, vec![doc!("_id" => "after")]).await;
    assert!(refused <= 30);
}

/// Every partition is fenced independently: writers on eight partitions keep writing through an
/// update, nothing is lost, and the update commits.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_concurrent_writers_on_many_partitions_during_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    for p in 0..8 {
        ctx.client
            .collection(&name)
            .partition(&format!("p{p}"))
            .upsert(vec![doc!("_id" => "seed", "rating" => 0u32)])
            .await
            .unwrap();
    }

    let writers = futures::future::join_all((0..8u32).map(|p| {
        let client = ctx.client.clone();
        let name = name.clone();
        async move {
            for i in 0..6u32 {
                client
                    .collection(&name)
                    .partition(&format!("p{p}"))
                    .upsert(vec![doc!("_id" => i.to_string(), "rating" => i)])
                    .await
                    .unwrap_or_else(|e| panic!("partition p{p} write {i}: {e:?}"));
            }
        }
    }));
    let collections = ctx.client.collections();
    let update = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let (_, updated) = tokio::join!(writers, update);
    assert!(updated.expect("every doc has a rating").schema["rating"].required);

    for p in 0..8 {
        let docs = ctx
            .client
            .collection(&name)
            .partition(&format!("p{p}"))
            .get(["seed", "0", "1", "2", "3", "4", "5"], None, None, None)
            .await
            .unwrap();
        assert_eq!(docs.len(), 7, "partition p{p}");
    }
}

/// One update may narrow many fields at once; every one of them is checked.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_many_fields_in_one_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    let mut doc = doc!("_id" => "a");
    for i in 0..100u32 {
        doc.fields.insert(format!("f{i}"), i.into());
    }
    upsert(ctx, &name, vec![doc]).await;

    let all: HashMap<String, FieldSpec> = (0..100)
        .map(|i| (format!("f{i}"), FieldSpec::integer(true)))
        .collect();
    let updated = update(ctx, &name, all.clone(), &[])
        .await
        .expect("every field has a value");
    assert_eq!(updated.schema.len(), 100);

    let mut one_wrong = all;
    one_wrong.insert("f42".into(), FieldSpec::text(true));
    let mut fresh = create_named(ctx, "fresh", HashMap::new()).await;
    let mut doc = doc!("_id" => "a");
    for i in 0..100u32 {
        doc.fields.insert(format!("f{i}"), i.into());
    }
    upsert(ctx, &fresh.name, vec![doc]).await;
    let msg = reason(update(ctx, &fresh.name, one_wrong, &[]).await.unwrap_err());
    assert!(
        msg.contains("field `f42`: stored values are Primitive(U32)"),
        "{msg}"
    );
    fresh = ctx.client.collections().get(&fresh.name).await.unwrap();
    assert!(fresh.schema.is_empty());
}

/// Five updates racing: at most one wins each CAS round, the rest are told to retry, and the
/// final schema is exactly the union of the winners.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_five_way_update_race(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    let mut doc = doc!("_id" => "a");
    for i in 0..5u32 {
        doc.fields.insert(format!("f{i}"), i.into());
    }
    upsert(ctx, &name, vec![doc]).await;

    let clients: Vec<_> = (0..5).map(|_| ctx.client.collections()).collect();
    let results = futures::future::join_all(clients.iter().enumerate().map(|(i, c)| {
        c.update(
            &name,
            schema!(format!("f{i}") => FieldSpec::integer(true)),
            vec![],
        )
    }))
    .await;

    let fetched = ctx.client.collections().get(&name).await.unwrap();
    let mut winners = 0;
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(_) => {
                winners += 1;
                assert!(fetched.schema[&format!("f{i}")].required);
            }
            Err(err) => {
                let msg = reason(err);
                assert!(
                    msg.contains("schema update in progress")
                        || msg.contains("concurrent schema update"),
                    "{msg}"
                );
                assert!(!fetched.schema.contains_key(&format!("f{i}")));
            }
        }
    }
    assert!(winners >= 1);
    assert_eq!(fetched.schema.len(), winners);
}

/// More partitions than `validation_concurrency`. Slow-ish: 100 partitions.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "100 partitions; run explicitly"]
async fn test_hundred_partitions(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    futures::future::join_all((0..100u32).map(|p| {
        let client = ctx.client.clone();
        let name = name.clone();
        async move {
            client
                .collection(&name)
                .partition(&format!("p{p}"))
                .upsert(vec![doc!("_id" => "a", "rating" => p)])
                .await
                .unwrap_or_else(|e| panic!("p{p}: {e:?}"));
        }
    }))
    .await;

    let started = std::time::Instant::now();
    let updated = update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .expect("every partition has a rating");
    assert!(updated.schema["rating"].required);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

/// A recreated collection is validated against its own data only; an empty request is a no-op.
/// Exposes a pre-existing writer bug: for up to `collection_cache` ttl after delete + recreate,
/// the writer still resolves the name to the deleted collection and the write is orphaned.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "pre-existing: writer serves the deleted collection until its cache expires"]
async fn test_recreated_collection_and_empty_request(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "a", "rating" => 1u32)]).await;
    ctx.client.collections().delete(&name).await.unwrap();

    let recreated = ctx
        .client
        .collections()
        .create(&name, HashMap::new(), None)
        .await
        .unwrap();
    let lsn = upsert(ctx, &name, vec![doc!("_id" => "b")]).await;
    let got = ctx
        .client
        .collection(&name)
        .get(["b"], None, Some(lsn), None)
        .await
        .unwrap();
    assert_eq!(got.len(), 1, "the write landed in the recreated collection");
    let msg = reason(
        update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(true)),
            &[],
        )
        .await
        .unwrap_err(),
    );
    assert!(
        msg.contains("field `rating`: null or missing in some rows"),
        "{msg}"
    );

    let same = update(ctx, &name, HashMap::new(), &[]).await.unwrap();
    assert_eq!(same.schema, recreated.schema);
}

/// Deeply nested fields flatten to one column each.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_deeply_nested_required(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    let doc = |id: &str, leaf: &str| {
        doc!(
            "_id" => id,
            "a" => Value::r#struct([("b", Value::r#struct([("c", leaf.into())]))]),
        )
    };
    upsert(ctx, &name, vec![doc("x", "1"), doc("y", "2")]).await;
    update(ctx, &name, schema!("a.b.c" => FieldSpec::text(true)), &[])
        .await
        .expect("every doc has a.b.c");

    let err = ctx
        .client
        .collection(&name)
        .upsert(vec![doc!("_id" => "z", "a" => Value::r#struct([("b", Value::r#struct([("d", "3".into())]))]))])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::DocumentValidationError(_)), "{err:?}");
}

/// Partitions born while an update runs: either the validator lists them and fences them, or
/// their first shard already starts on the pending schema. Either way the new rule holds.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_new_partitions_during_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "seed", "rating" => 0u32)]).await;

    let writers = futures::future::join_all((0..8u32).map(|p| {
        let client = ctx.client.clone();
        let name = name.clone();
        async move {
            for i in 0..4u32 {
                client
                    .collection(&name)
                    .partition(&format!("fresh{p}"))
                    .upsert(vec![doc!("_id" => i.to_string(), "rating" => i)])
                    .await
                    .unwrap_or_else(|e| panic!("fresh{p} write {i}: {e:?}"));
            }
        }
    }));
    let collections = ctx.client.collections();
    let update = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let (_, updated) = tokio::join!(writers, update);
    assert!(updated.expect("every doc has a rating").schema["rating"].required);

    for p in 0..8 {
        let docs = ctx
            .client
            .collection(&name)
            .partition(&format!("fresh{p}"))
            .get(["0", "1", "2", "3"], None, None, None)
            .await
            .unwrap();
        assert_eq!(docs.len(), 4, "fresh{p}");
        let err = ctx
            .client
            .collection(&name)
            .partition(&format!("fresh{p}"))
            .upsert(vec![doc!("_id" => "late")])
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::DocumentValidationError(_)),
            "fresh{p}: {err:?}"
        );
    }
}

/// A writer coming up on a WAL whose newest segment is a barrier starts its shard right after
/// it. Emulator only: restarts the writer deployment.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "restarts the emulator writer; run alone"]
async fn test_fresh_writer_after_barriers(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;
    update(
        ctx,
        &name,
        schema!("rating" => FieldSpec::integer(true)),
        &[],
    )
    .await
    .unwrap();

    for args in [
        vec!["rollout", "restart", "deployment/writer"],
        vec!["rollout", "status", "deployment/writer", "--timeout=120s"],
    ] {
        let status = std::process::Command::new("kubectl")
            .args(["--context", "k3d-ddb-cluster", "-n", "ddb-writer"])
            .args(&args)
            .status()
            .expect("kubectl");
        assert!(status.success(), "{args:?}");
    }

    let mut lsn = None;
    for attempt in 0..20 {
        match ctx
            .client
            .collection(&name)
            .upsert(vec![
                doc!("_id" => "late", "title" => "Late", "summary" => "s", "rating" => 1u32),
            ])
            .await
        {
            Ok(l) => {
                lsn = Some(l);
                break;
            }
            Err(_) if attempt < 19 => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => panic!("{e:?}"),
        }
    }
    let got = ctx
        .client
        .collection(&name)
        .get(["late", "pride"], None, lsn, None)
        .await
        .unwrap();
    assert_eq!(got.len(), 2);
}

/// Narrowing an indexed field keeps working as long as the index itself is unchanged; indexed
/// vectors are stored as dense matrices and checked against the declared dimension.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_required_on_indexed_fields(ctx: &mut ProjectTestContext) {
    let vector = |required: bool| {
        FieldSpec::f32_vector(3, required)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine))
    };
    let collection = create(
        ctx,
        schema!("title" => keyword(FieldSpec::text(false)), "emb" => vector(false)),
    )
    .await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![
            doc!("_id" => "a", "title" => "one", "emb" => vec![1.0f32, 0.0, 0.0]),
            doc!("_id" => "b", "title" => "two", "emb" => vec![0.0f32, 1.0, 0.0]),
        ],
    )
    .await;

    let updated = update(
        ctx,
        &name,
        schema!("title" => keyword(FieldSpec::text(true)), "emb" => vector(true)),
        &[],
    )
    .await
    .expect("index unchanged, every doc has both");
    assert!(updated.schema["title"].required && updated.schema["emb"].required);

    let err = ctx
        .client
        .collection(&name)
        .upsert(vec![doc!("_id" => "c", "title" => "three")])
        .await
        .unwrap_err();
    assert!(matches!(err, Error::DocumentValidationError(_)), "{err:?}");

    let msg = reason(
        update(
            ctx,
            &name,
            schema!("emb" => FieldSpec::f32_vector(4, true).with_index(FieldIndex::vector(VectorDistanceMetric::Cosine))),
            &[],
        )
        .await
        .unwrap_err(),
    );
    assert!(
        msg.contains("field `emb`: type changes unsupported"),
        "{msg}"
    );
}

/// The blob store vanishing mid-update: the update fails, the pending record is rolled back, and
/// the next update goes through once storage is back. Emulator only: pauses the localstack
/// container.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "pauses the emulator's localstack; run alone"]
async fn test_update_survives_a_blob_store_outage(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;

    let docker = |verb: &str| {
        let status = std::process::Command::new("docker")
            .args([verb, "ddb-emulator-localstack-1"])
            .status()
            .expect("docker");
        assert!(status.success(), "docker {verb}");
    };
    docker("pause");
    let collections = ctx.client.collections();
    let during = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let outage = tokio::time::timeout(std::time::Duration::from_secs(40), during).await;
    docker("unpause");
    match outage {
        Ok(Ok(_)) => panic!("update cannot validate without the blob store"),
        Ok(Err(err)) => assert!(!matches!(err, Error::DocumentValidationError(_)), "{err:?}"),
        Err(_) => {}
    }

    let mut last = None;
    for _ in 0..12 {
        match update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(true)),
            &[],
        )
        .await
        {
            Ok(updated) => {
                assert!(updated.schema["rating"].required);
                return;
            }
            Err(err) => {
                last = Some(err);
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
    panic!("update never recovered: {last:?}");
}

/// Partial updates and deletes racing a narrowing go through the same fenced commit loop as
/// upserts: none is lost, the update commits, and the merged documents satisfy the new rule.
#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_partial_updates_and_deletes_during_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    let seed: Vec<Document> = (0..40u32)
        .map(|i| doc!("_id" => i.to_string(), "rating" => i, "title" => format!("t{i}")))
        .collect();
    upsert(ctx, &name, seed).await;

    let writer = ctx.client.clone();
    let writes = async {
        for i in 0..20u32 {
            writer
                .collection(&name)
                .update(
                    vec![doc!("_id" => i.to_string(), "title" => format!("edited{i}"))],
                    false,
                )
                .await
                .unwrap_or_else(|e| panic!("update {i}: {e:?}"));
            writer
                .collection(&name)
                .delete(vec![(20 + i).to_string()])
                .await
                .unwrap_or_else(|e| panic!("delete {}: {e:?}", 20 + i));
        }
    };
    let collections = ctx.client.collections();
    let update = collections.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
    let (_, updated) = tokio::join!(writes, update);
    assert!(updated.expect("every remaining doc has a rating").schema["rating"].required);

    let ids: Vec<String> = (0..40).map(|i| i.to_string()).collect();
    let docs = ctx
        .client
        .collection(&name)
        .get(
            ids.iter().map(String::as_str).collect::<Vec<_>>(),
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 20);
    for i in 0..20 {
        assert_eq!(docs[&i.to_string()]["title"], format!("edited{i}").into());
    }
}

/// Model-based fuzz: random writes, deletes, schema updates and races against one collection
/// spread over a few partitions, with flat and nested fields. A local model predicts every
/// outcome; the only slack is a conflict that lives in dead rows compaction may have rewritten.
/// `FUZZ_SEED` / `FUZZ_STEPS`.
#[test_context(ProjectTestContext)]
#[tokio::test]
#[ignore = "randomized; run explicitly, FUZZ_SEED / FUZZ_STEPS"]
async fn fuzz_schema_updates(ctx: &mut ProjectTestContext) {
    use rand::{
        rngs::StdRng,
        seq::{IteratorRandom, SliceRandom},
        Rng, SeedableRng,
    };
    use std::collections::HashSet;
    use topk_rs::proto::v1::control::FieldType;

    fn string_list(required: bool) -> FieldSpec {
        FieldSpec::list(required, ListValueType::String)
    }
    // every field has a natural type; docs mostly hold it, sometimes another one. `m.*` are
    // nested, and a doc sometimes holds a scalar `m` instead.
    const FIELDS: [(&str, fn(bool) -> FieldSpec); 8] = [
        ("a", FieldSpec::text),
        ("b", FieldSpec::integer),
        ("c", FieldSpec::float),
        ("d", FieldSpec::boolean),
        ("e", FieldSpec::bytes),
        ("f", string_list),
        ("m.t", FieldSpec::text),
        ("m.n", FieldSpec::integer),
    ];
    fn parent(field: &str) -> Option<&str> {
        field.rsplit_once('.').map(|(parent, _)| parent)
    }
    const PARTITIONS: [&str; 3] = ["", "p1", "p2"];

    fn type_of(value: &Value) -> FieldType {
        if value.as_string().is_some() {
            FieldType::text()
        } else if value.as_u32().is_some() {
            FieldType::integer()
        } else if value.as_f64().is_some() {
            FieldType::float()
        } else if value.as_bool().is_some() {
            FieldType::boolean()
        } else if value.as_binary().is_some() {
            FieldType::bytes()
        } else if value.as_string_list().is_some() {
            FieldType::list(ListValueType::String)
        } else {
            panic!("unexpected value {value:?}")
        }
    }
    fn natural_type(field: &str) -> FieldType {
        let spec = FIELDS.iter().find(|(f, _)| *f == field).unwrap().1;
        spec(false).data_type.unwrap()
    }
    fn random_type(rng: &mut StdRng) -> FieldType {
        natural_type(FIELDS.choose(rng).unwrap().0)
    }
    fn random_value(rng: &mut StdRng, t: &FieldType) -> Value {
        let n = rng.gen_range(0..1000u32);
        if *t == FieldType::text() {
            format!("s{n}").into()
        } else if *t == FieldType::integer() {
            n.into()
        } else if *t == FieldType::float() {
            (n as f64 / 8.0).into()
        } else if *t == FieldType::boolean() {
            (n % 2 == 0).into()
        } else if *t == FieldType::bytes() {
            Value::binary(n.to_le_bytes().to_vec())
        } else {
            vec![format!("l{n}")].into()
        }
    }
    // docs are modelled flat, `m.t` as a dotted key; a scalar parent is the key `m` itself
    fn random_doc(rng: &mut StdRng) -> HashMap<String, Value> {
        let mut doc = HashMap::new();
        for (f, _) in FIELDS {
            if rng.gen_bool(0.95) {
                let t = if rng.gen_bool(0.97) {
                    natural_type(f)
                } else {
                    random_type(rng)
                };
                doc.insert(f.to_string(), random_value(rng, &t));
            }
        }
        if rng.gen_bool(0.03) {
            doc.retain(|f, _| parent(f) != Some("m"));
            doc.insert("m".to_string(), random_value(rng, &FieldType::text()));
        }
        doc
    }
    fn document(id: &str, fields: &HashMap<String, Value>) -> Document {
        let mut doc = doc!("_id" => id);
        let mut structs: HashMap<String, HashMap<String, Value>> = HashMap::new();
        for (f, v) in fields {
            match f.split_once('.') {
                Some((parent, child)) => {
                    structs
                        .entry(parent.to_string())
                        .or_default()
                        .insert(child.to_string(), v.clone());
                }
                None => {
                    doc.fields.insert(f.clone(), v.clone());
                }
            }
        }
        for (parent, children) in structs {
            doc.fields.insert(parent, children.into());
        }
        doc
    }
    fn flatten(fields: &HashMap<String, Value>) -> HashMap<String, Value> {
        let mut flat = HashMap::new();
        for (f, v) in fields {
            match v.as_struct() {
                Some(children) => {
                    for (child, cv) in children {
                        flat.insert(format!("{f}.{child}"), cv.clone());
                    }
                }
                None => {
                    flat.insert(f.clone(), v.clone());
                }
            }
        }
        flat
    }

    #[derive(Default)]
    struct Model {
        schema: HashMap<String, FieldSpec>,
        // id -> (partition, fields)
        live: HashMap<String, (String, HashMap<String, Value>)>,
        // fields some stored row lacks / holds with another type / holds a scalar where the
        // field's parent should be; rows outlive their docs
        missing: HashSet<String>,
        mistyped: HashSet<String>,
        scalar_parents: HashSet<String>,
    }
    impl Model {
        fn violates(&self, doc: &HashMap<String, Value>) -> bool {
            self.schema.iter().any(|(f, spec)| match doc.get(f) {
                Some(v) => Some(type_of(v)) != spec.data_type,
                None => spec.required,
            })
        }
        fn record(&mut self, id: String, partition: &str, doc: HashMap<String, Value>) {
            for (f, _) in FIELDS {
                match doc.get(f) {
                    None => {
                        self.missing.insert(f.to_string());
                    }
                    Some(v) if type_of(v) != natural_type(f) => {
                        self.mistyped.insert(f.to_string());
                    }
                    _ => {}
                }
                if let Some(p) = parent(f).filter(|p| doc.contains_key(*p)) {
                    self.scalar_parents.insert(p.to_string());
                }
            }
            self.live.insert(id, (partition.to_string(), doc));
        }
        // whatever landed is the truth, and proved its newly narrowed fields clean
        fn resync(&mut self, schema: HashMap<String, FieldSpec>) {
            for (f, spec) in &schema {
                let old = self.schema.get(f);
                if old.is_none() {
                    self.mistyped.remove(f);
                    if let Some(p) = parent(f) {
                        self.scalar_parents.remove(p);
                    }
                }
                if spec.required && !old.is_some_and(|o| o.required) {
                    self.missing.remove(f);
                }
            }
            self.schema = schema;
        }
        fn docs_in(&self, partition: &str) -> Vec<String> {
            self.live
                .iter()
                .filter(|(_, (p, _))| p == partition)
                .map(|(id, _)| id.clone())
                .collect()
        }
    }

    // the error must contain one of `needles`; none means acceptance. `certain` is false when the
    // only conflicts live in dead rows, so acceptance is fine too.
    #[derive(Debug)]
    struct Expect {
        needles: Vec<String>,
        certain: bool,
    }
    fn predict(model: &Model, request: &HashMap<String, FieldSpec>, drops: &[String]) -> Expect {
        let mut plan = vec![];
        for f in drops {
            if !model.schema.contains_key(f) {
                plan.push(format!("field `{f}`: unknown"));
            }
            if request.contains_key(f) {
                plan.push(format!("field `{f}`: updated and dropped"));
            }
        }
        for (f, spec) in request {
            if model
                .schema
                .get(f)
                .is_some_and(|old| old.data_type != spec.data_type)
            {
                plan.push(format!("field `{f}`: type changes unsupported"));
            }
        }
        if !plan.is_empty() {
            return Expect {
                needles: plan,
                certain: true,
            };
        }

        let mut widens = drops.iter().any(|f| model.schema.contains_key(f));
        let mut narrows = false;
        let (mut live, mut dead) = (vec![], vec![]);
        for (f, spec) in request {
            let old = model.schema.get(f);
            let gains_type = old.is_none();
            let gains_required = spec.required && !old.is_some_and(|o| o.required);
            widens |= old.is_some_and(|o| o.required && !spec.required);
            narrows |= gains_type || gains_required;
            let needle = format!("field `{f}`:");
            let docs = model.live.values().map(|(_, d)| d);
            if gains_type {
                if let Some(p) = parent(f) {
                    if docs.clone().any(|d| d.contains_key(p)) {
                        live.push(needle.clone());
                    } else if model.scalar_parents.contains(p) {
                        dead.push(needle.clone());
                    }
                }
                if docs
                    .clone()
                    .any(|d| d.get(f).is_some_and(|v| Some(type_of(v)) != spec.data_type))
                {
                    live.push(needle.clone());
                } else if model.mistyped.contains(f) {
                    dead.push(needle.clone());
                }
            }
            if gains_required {
                if docs.clone().any(|d| !d.contains_key(f)) {
                    live.push(needle);
                } else if model.missing.contains(f) {
                    dead.push(needle);
                }
            }
        }
        if widens && narrows {
            return Expect {
                needles: vec!["cannot narrow and widen in one update".into()],
                certain: true,
            };
        }
        let certain = !live.is_empty() || dead.is_empty();
        live.extend(dead);
        Expect {
            needles: live,
            certain,
        }
    }
    // mostly well-formed: narrow (declare or tighten) or widen (drop or relax); sometimes a
    // request the planner refuses
    fn random_request(
        rng: &mut StdRng,
        model: &Model,
    ) -> (HashMap<String, FieldSpec>, Vec<String>) {
        let mut schema = HashMap::new();
        let mut drops = vec![];
        let declared: Vec<&String> = model.schema.keys().collect();
        match rng.gen_range(0..10) {
            0..=6 => {
                for (f, spec) in FIELDS {
                    if rng.gen_bool(0.6) {
                        continue;
                    }
                    let everywhere = model.live.values().all(|(_, d)| d.contains_key(f));
                    let required = model.schema.get(f).is_some_and(|old| old.required)
                        || everywhere && rng.gen_bool(0.7);
                    schema.insert(f.to_string(), spec(required));
                }
            }
            7..=8 => {
                for f in &declared {
                    if rng.gen_bool(0.5) {
                        continue;
                    }
                    let old = &model.schema[*f];
                    if old.required && rng.gen_bool(0.5) {
                        schema.insert(
                            f.to_string(),
                            FieldSpec {
                                required: false,
                                ..old.clone()
                            },
                        );
                    } else {
                        drops.push(f.to_string());
                    }
                }
            }
            _ => match (rng.gen_range(0..4), declared.choose(rng)) {
                (1, Some(f)) => {
                    schema.insert(f.to_string(), FieldSpec::text(false));
                    drops.push(f.to_string());
                }
                (2, Some(f)) => {
                    let mut spec = model.schema[*f].clone();
                    while spec.data_type == model.schema[*f].data_type {
                        spec.data_type = Some(random_type(rng));
                    }
                    schema.insert(f.to_string(), spec);
                }
                (3, Some(f)) => {
                    drops.push(f.to_string());
                    let (g, spec) = FIELDS
                        .iter()
                        .find(|(g, _)| !model.schema.contains_key(*g))
                        .unwrap_or(&FIELDS[0]);
                    schema.insert(g.to_string(), spec(false));
                }
                _ => drops.push("zzz".to_string()),
            },
        }
        (schema, drops)
    }

    let seed = std::env::var("FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(rand::random::<u64>);
    let steps: usize = std::env::var("FUZZ_STEPS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);
    println!("FUZZ_SEED={seed} FUZZ_STEPS={steps}");
    let mut rng = StdRng::seed_from_u64(seed);

    let name = create(ctx, HashMap::new()).await.name;
    let client = ctx.client.clone();
    let collection = |partition: &str| match partition {
        "" => client.collection(&name),
        p => client.collection(&name).partition(p),
    };
    let mut model = Model::default();
    let mut next_id = 0u32;
    // last write lsn per partition; each partition has its own wal
    let mut lsn: HashMap<String, String> = HashMap::new();

    for step in 0..steps {
        match rng.gen_range(0..100) {
            // upsert 1..=3 docs into one partition
            0..=34 => {
                let partition = *PARTITIONS.choose(&mut rng).unwrap();
                let existing = model.docs_in(partition);
                let docs: Vec<(String, HashMap<String, Value>)> = (0..rng.gen_range(1..=3))
                    .map(|_| {
                        let id = match existing.choose(&mut rng) {
                            Some(id) if rng.gen_bool(0.3) => id.clone(),
                            _ => {
                                next_id += 1;
                                format!("d{next_id}")
                            }
                        };
                        (id, random_doc(&mut rng))
                    })
                    .collect();
                let violates = docs.iter().any(|(_, d)| model.violates(d));
                let result = collection(partition)
                    .upsert(docs.iter().map(|(id, d)| document(id, d)).collect())
                    .await;
                println!(
                    "{step}: upsert {} docs into {partition:?} violates={violates} -> {}",
                    docs.len(),
                    result.is_ok()
                );
                if violates {
                    assert!(
                        matches!(result, Err(Error::DocumentValidationError(_))),
                        "step {step}: {result:?}"
                    );
                } else {
                    lsn.insert(
                        partition.to_string(),
                        result.unwrap_or_else(|e| panic!("step {step}: {e:?}")),
                    );
                    for (id, doc) in docs {
                        model.record(id, partition, doc);
                    }
                }
            }
            // partial update of one field of an existing doc
            35..=44 if !model.live.is_empty() => {
                let (id, (partition, doc)) = model.live.iter().choose(&mut rng).unwrap();
                let (id, partition, mut merged) = (id.clone(), partition.clone(), doc.clone());
                let f = FIELDS
                    .iter()
                    .filter(|(f, _)| parent(f).is_none())
                    .choose(&mut rng)
                    .unwrap()
                    .0;
                let t = if rng.gen_bool(0.97) {
                    natural_type(f)
                } else {
                    random_type(&mut rng)
                };
                merged.insert(f.to_string(), random_value(&mut rng, &t));
                let violates = model.violates(&merged);
                let patch = HashMap::from([(f.to_string(), merged[f].clone())]);
                let result = collection(&partition)
                    .update(vec![document(&id, &patch)], true)
                    .await;
                println!(
                    "{step}: patch {id}.{f} violates={violates} -> {}",
                    result.is_ok()
                );
                if violates {
                    assert!(
                        matches!(result, Err(Error::DocumentValidationError(_))),
                        "step {step}: {result:?}"
                    );
                } else {
                    lsn.insert(
                        partition.clone(),
                        result.unwrap_or_else(|e| panic!("step {step}: {e:?}")),
                    );
                    model.record(id, &partition, merged);
                }
            }
            // delete one doc
            45..=54 if !model.live.is_empty() => {
                let (id, (partition, _)) = model.live.iter().choose(&mut rng).unwrap();
                let (id, partition) = (id.clone(), partition.clone());
                lsn.insert(
                    partition.clone(),
                    collection(&partition)
                        .delete(vec![id.clone()])
                        .await
                        .unwrap_or_else(|e| panic!("step {step}: {e:?}")),
                );
                println!("{step}: delete {id}");
                model.live.remove(&id);
            }
            // schema update with a predicted outcome
            55..=82 => {
                let (request, drops) = random_request(&mut rng, &model);
                let expect = predict(&model, &request, &drops);
                let mut expected_schema = model.schema.clone();
                expected_schema.retain(|f, _| !drops.contains(f));
                expected_schema.extend(request.clone());
                let result = client
                    .collections()
                    .update(&name, request.clone(), drops.clone())
                    .await;
                println!(
                    "{step}: update {request:?} drop {drops:?} expect {expect:?} -> {:?}",
                    result.as_ref().map(|c| c.schema.len())
                );
                match result {
                    Ok(updated) if expect.needles.is_empty() || !expect.certain => {
                        assert_eq!(updated.schema, expected_schema, "step {step}");
                        model.resync(updated.schema);
                    }
                    Err(err) if !expect.needles.is_empty() => {
                        let msg = reason(err);
                        assert!(
                            expect.needles.iter().any(|n| msg.contains(n)),
                            "step {step}: {msg:?} has none of {:?}",
                            expect.needles
                        );
                        let fetched = client.collections().get(&name).await.unwrap();
                        assert_eq!(
                            fetched.schema, model.schema,
                            "step {step}: schema after rejection"
                        );
                    }
                    other => panic!("step {step}: expected {expect:?}, got {other:?}"),
                }
            }
            // 10 writes racing one update: whatever landed satisfies the schema that won
            83..=89 => {
                let (request, drops) = random_request(&mut rng, &model);
                let burst: Vec<(String, &str, HashMap<String, Value>)> = (0..10)
                    .map(|_| {
                        next_id += 1;
                        (
                            format!("d{next_id}"),
                            *PARTITIONS.choose(&mut rng).unwrap(),
                            random_doc(&mut rng),
                        )
                    })
                    .collect();
                let writes = futures::future::join_all(burst.iter().map(|(id, p, doc)| {
                    let collection = collection(p);
                    async move { collection.upsert(vec![document(id, doc)]).await }
                }));
                let collections = client.collections();
                let update = collections.update(&name, request.clone(), drops.clone());
                let (writes, updated) = tokio::join!(writes, update);
                let fetched = client.collections().get(&name).await.unwrap();
                model.resync(fetched.schema);
                let mut landed = 0;
                for (result, (id, partition, doc)) in writes.into_iter().zip(burst) {
                    match result {
                        Ok(write_lsn) => {
                            landed += 1;
                            lsn.insert(partition.to_string(), write_lsn);
                            assert!(
                                !model.violates(&doc),
                                "step {step}: {id} landed with {doc:?} against {:?}",
                                model.schema
                            );
                            model.record(id, partition, doc);
                        }
                        Err(Error::DocumentValidationError(_)) => {}
                        Err(e) => panic!("step {step}: write {id}: {e:?}"),
                    }
                }
                println!("{step}: burst with update {request:?} drop {drops:?} -> update {} landed {landed}", if updated.is_ok() { "ok" } else { "rejected" });
                if let Err(err) = updated {
                    let msg = reason(err);
                    assert!(
                        msg.contains("field `") || msg.contains("cannot narrow"),
                        "step {step}: {msg}"
                    );
                }
            }
            // 2..=3 updates racing: the schema is the last winner's, losers were told why
            90..=94 => {
                let requests: Vec<_> = (0..rng.gen_range(2..=3))
                    .map(|_| random_request(&mut rng, &model))
                    .collect();
                let clients: Vec<_> = requests.iter().map(|_| client.collections()).collect();
                let results =
                    futures::future::join_all(clients.iter().zip(&requests).map(
                        |(c, (request, drops))| c.update(&name, request.clone(), drops.clone()),
                    ))
                    .await;
                let fetched = client.collections().get(&name).await.unwrap();
                println!(
                    "{step}: race of {}: {:?}",
                    requests.len(),
                    results.iter().map(Result::is_ok).collect::<Vec<_>>()
                );
                let mut winners = 0;
                for result in results {
                    match result {
                        Ok(_) => winners += 1,
                        Err(err) => {
                            let msg = reason(err);
                            assert!(
                                msg.contains("field `")
                                    || msg.contains("cannot narrow")
                                    || msg.contains("in progress")
                                    || msg.contains("concurrent"),
                                "step {step}: {msg}"
                            );
                        }
                    }
                }
                if winners == 0 {
                    assert_eq!(
                        fetched.schema, model.schema,
                        "step {step}: no winner, schema unchanged"
                    );
                }
                model.resync(fetched.schema);
            }
            // read every live doc back, per partition
            _ => {
                for partition in PARTITIONS {
                    let ids = model.docs_in(partition);
                    if ids.is_empty() {
                        continue;
                    }
                    let got = collection(partition)
                        .get(ids.clone(), None, lsn.get(partition).cloned(), None)
                        .await
                        .unwrap_or_else(|e| panic!("step {step}: {e:?}"));
                    println!("{step}: read {} docs from {partition:?}", ids.len());
                    let mut missing: Vec<_> =
                        ids.iter().filter(|id| !got.contains_key(*id)).collect();
                    missing.sort();
                    assert!(missing.is_empty(), "step {step}: not returned: {missing:?}");
                    for id in &ids {
                        let mut flat = flatten(&got[id]);
                        flat.remove("_id");
                        assert_eq!(flat, model.live[id].1, "step {step}: {id}");
                    }
                }
            }
        }
    }
    let fetched = client.collections().get(&name).await.unwrap();
    assert_eq!(fetched.schema, model.schema, "final schema");
}
