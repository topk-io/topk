use std::collections::HashMap;
use std::time::{Duration, Instant};

use test_context::test_context;
use topk_rs::error::SchemaValidationError;
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
    ssn: Option<u32>,
) -> Result<Vec<String>, Error> {
    let docs = ctx
        .client
        .collection(collection)
        .query(
            filter(r#match(term, Some(fieldname), None, false))
                .select([("title", field("title"))])
                .limit(100),
            lsn,
            ssn,
            None,
        )
        .await?;
    let mut ids: Vec<String> = docs.iter().map(|d| d.id().unwrap().to_string()).collect();
    ids.sort();
    Ok(ids)
}

async fn wait_for_index<T>(probe: impl AsyncFn() -> Result<T, Error>) -> T {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match probe().await {
            Ok(value) => return value,
            Err(Error::IndexBuilding(_)) => {}
            Err(err) => panic!("query on a building index: {err:?}"),
        }
        assert!(Instant::now() < deadline, "index still building after 120s");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn write_through_pause<T>(write: impl AsyncFn() -> Result<T, Error>) -> T {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        match write().await {
            Ok(value) => return value,
            Err(Error::SlowDown(msg)) if msg.contains("schema update in progress") => {}
            Err(err) => panic!("write during a schema update: {err:?}"),
        }
        assert!(Instant::now() < deadline, "still paused after 120s");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn reason(err: Error) -> String {
    match err {
        Error::FailedPrecondition(msg) | Error::InvalidArgument(msg) => msg,
        e => panic!("{e:?}"),
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_add_index_on_written_field(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => keyword(FieldSpec::text(true)))).await;
    let name = collection.name.clone();
    ctx.client.collection(&name).upsert(docs()).await.unwrap();

    // `summary` is stored but undeclared: no keyword index to match on
    ids_matching(ctx, &name, "love", "summary", None, None)
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
        wait_for_index(async || ids_matching(ctx, &name, "love", "summary", None, None).await).await;
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
    let ids = ids_matching(ctx, &name, "love", "summary", Some(lsn), None)
        .await
        .unwrap();
    assert_eq!(ids, vec!["gatsby", "new", "pride"]);
}

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
        ids_matching(ctx, &name, "love", "summary", Some(lsn), None)
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
    loop {
        match ids_matching(ctx, &name, "love", "summary", None, None).await {
            Err(Error::InvalidArgument(_)) => break,
            Ok(_) | Err(Error::IndexBuilding(_)) => {}
            Err(err) => panic!("query after index drop: {err:?}"),
        }
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].id().unwrap(), "moby");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_drop_an_index_immediately_after_adding_it(ctx: &mut ProjectTestContext) {
    let created = create(
        ctx,
        schema!("title" => keyword(FieldSpec::text(true)), "summary" => FieldSpec::text(false)),
    )
    .await;
    let name = created.name;
    let lsn = ctx.client.collection(&name).upsert(docs()).await.unwrap();
    assert!(
        !ids_matching(ctx, &name, "gatsby", "title", Some(lsn.clone()), None)
            .await
            .unwrap()
            .is_empty()
    );
    update(
        ctx,
        &name,
        schema!("summary" => keyword(FieldSpec::text(false))),
        &[],
    )
    .await
    .unwrap();
    let dropped = update(
        ctx,
        &name,
        schema!("summary" => FieldSpec::text(false)),
        &[],
    )
    .await
    .unwrap();
    let result = ctx
        .client
        .collection(&name)
        .query(
            select([("summary", field("summary"))]).limit(100),
            Some(lsn.clone()),
            Some(dropped.ssn),
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.len(), docs().len());
    assert!(result.iter().all(|doc| doc.fields.contains_key("summary")));
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
            None,
        )
        .await?;
    Ok(docs.iter().map(|d| d.id().unwrap().to_string()).collect())
}

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

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_read_after_dropping_a_vector_index(ctx: &mut ProjectTestContext) {
    let collection = create(
        ctx,
        schema!("vector" => FieldSpec::f32_vector(2, false)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean))),
    )
    .await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "indexed", "vector" => vec![1.0f32, 0.0])],
    )
    .await;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let updated = update(
        ctx,
        &name,
        schema!("vector" => FieldSpec::f32_vector(2, false)),
        &[],
    )
    .await
    .unwrap();
    assert!(updated.schema["vector"].index.is_none());

    let lsn = upsert(
        ctx,
        &name,
        vec![doc!("_id" => "plain", "vector" => vec![0.0f32, 1.0])],
    )
    .await;

    let docs = ctx
        .client
        .collection(&name)
        .get(
            ["indexed", "plain"],
            Some(vec!["vector".to_string()]),
            Some(lsn),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 2);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_update_after_dropping_a_vector_index(ctx: &mut ProjectTestContext) {
    let collection = create(
        ctx,
        schema!("vector" => FieldSpec::f32_vector(2, false)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean))),
    )
    .await;
    let name = collection.name.clone();
    let lsn = upsert(
        ctx,
        &name,
        vec![doc!("_id" => "indexed", "vector" => vec![1.0f32, 0.0])],
    )
    .await;

    let indexed = ctx
        .client
        .collection(&name)
        .query(
            select([("dist", fns::vector_distance("vector", vec![1.0f32, 0.0]))])
                .sort([(field("dist"), SortOrder::Asc)])
                .limit(1),
            Some(lsn),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(indexed.len(), 1);
    assert_eq!(indexed[0].id().unwrap(), "indexed");

    update(
        ctx,
        &name,
        schema!("vector" => FieldSpec::f32_vector(2, false)),
        &[],
    )
    .await
    .unwrap();

    let updated = update(
        ctx,
        &name,
        schema!("vector" => FieldSpec::f32_vector(2, true)),
        &[],
    )
    .await
    .unwrap();
    assert!(updated.schema["vector"].required);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_writes_during_a_vector_index_add(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("vector" => FieldSpec::f32_vector(2, false))).await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "seed", "vector" => vec![1.0f32, 0.0])],
    )
    .await;

    let writer = ctx.client.clone();
    let writes = async {
        let mut lsn = None;
        for i in 0..30u32 {
            lsn = Some(
                write_through_pause(async || {
                    writer
                        .collection(&name)
                        .upsert(vec![
                            doc!("_id" => i.to_string(), "vector" => vec![0.0f32, 1.0]),
                        ])
                        .await
                })
                .await,
            );
        }
        lsn
    };
    let collections = ctx.client.collections();
    let update = collections.update(
        &name,
        schema!("vector" => FieldSpec::f32_vector(2, false)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean))),
        vec![],
    );
    let (lsn, updated) = tokio::join!(writes, update);
    updated.expect("index add");

    assert_eq!(
        wait_for_index(async || nearest(ctx, &name).await)
            .await
            .len(),
        2
    );

    let docs = ctx
        .client
        .collection(&name)
        .get(["seed"], None, lsn, None, None)
        .await
        .unwrap();
    assert_eq!(docs.len(), 1);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_a_refused_update_stays_refused_under_concurrency(ctx: &mut ProjectTestContext) {
    for attempt in 0..8u32 {
        let collection = create_named(ctx, &format!("race{attempt}"), HashMap::new()).await;
        let name = collection.name.clone();
        upsert(ctx, &name, docs()).await;
        upsert(
            ctx,
            &name,
            vec![doc!("_id" => "unrated", "title" => "No Rating")],
        )
        .await;

        let a = ctx.client.collections();
        let b = ctx.client.collections();
        let refusable = a.update(&name, schema!("rating" => FieldSpec::integer(true)), vec![]);
        let benign = b.update(&name, schema!("summary" => FieldSpec::text(false)), vec![]);
        let (refused, _) = tokio::join!(refusable, benign);
        assert!(
            refused.is_err(),
            "attempt {attempt}: `rating` has no value in `unrated`, but the update committed"
        );

        let visible = ctx.client.collections().get(&name).await.unwrap();
        assert!(
            !visible.schema.get("rating").is_some_and(|s| s.required),
            "attempt {attempt}: refused declaration landed in the schema anyway"
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_a_vector_index_converges_in_every_partition(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("vector" => FieldSpec::f32_vector(2, false))).await;
    let name = collection.name.clone();
    for (partition, id, vector) in [
        (None, "root", vec![1.0f32, 0.0]),
        (Some("p1"), "one", vec![0.0f32, 1.0]),
        (Some("p2"), "two", vec![0.7f32, 0.7]),
    ] {
        let doc = vec![doc!("_id" => id, "vector" => vector)];
        match partition {
            Some(p) => ctx
                .client
                .collection(&name)
                .partition(p)
                .upsert(doc)
                .await
                .unwrap(),
            None => ctx.client.collection(&name).upsert(doc).await.unwrap(),
        };
    }

    // outlast the emulator's flush interval so every partition holds the field in a file
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    update(
        ctx,
        &name,
        schema!("vector" => FieldSpec::f32_vector(2, false)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean))),
        &[],
    )
    .await
    .expect("index add");

    for (partition, expected) in [(None, "root"), (Some("p1"), "one"), (Some("p2"), "two")] {
        let ids = wait_for_index(async || {
            let query = select([("dist", fns::vector_distance("vector", vec![1.0f32, 0.0]))])
                .sort([(field("dist"), SortOrder::Asc)])
                .limit(1);
            let docs = match partition {
                Some(p) => {
                    ctx.client
                        .collection(&name)
                        .partition(p)
                        .query(query, None, None, None)
                        .await?
                }
                None => {
                    ctx.client
                        .collection(&name)
                        .query(query, None, None, None)
                        .await?
                }
            };
            Ok(docs
                .iter()
                .map(|d| d.id().unwrap().to_string())
                .collect::<Vec<_>>())
        })
        .await;
        assert_eq!(ids, vec![expected.to_string()], "partition {partition:?}");
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_an_unindexed_vector_dimension_is_validated(ctx: &mut ProjectTestContext) {
    let bad = ctx.wrap("dims");
    let errors = ctx
        .client
        .collections()
        .create(&bad, schema!("zero" => FieldSpec::f32_vector(0, false), "huge" => FieldSpec::u8_vector(100_000, false)), None)
        .await
        .expect_err("an unindexed vector still has to have a usable dimension");
    let Error::SchemaValidationError(errors) = errors else {
        panic!("{errors:?}")
    };
    let errors: Vec<_> = errors.into_iter().collect();
    assert!(
        errors.contains(&SchemaValidationError::VectorDimensionCannotBeZero {
            field: "zero".to_string()
        })
    );
    assert!(
        errors.contains(&SchemaValidationError::VectorDimensionTooLarge {
            field: "huge".to_string(),
            dimension: 100_000
        })
    );

    let collection = create(ctx, schema!("title" => FieldSpec::text(false))).await;
    let name = collection.name.clone();
    upsert(ctx, &name, vec![doc!("_id" => "a", "title" => "x")]).await;

    let errors = update(
        ctx,
        &name,
        schema!("later" => FieldSpec::f32_vector(0, false)),
        &[],
    )
    .await
    .expect_err("an update cannot declare a shape create would refuse");
    assert!(
        matches!(errors, Error::SchemaValidationError(_)),
        "{errors:?}"
    );
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_a_scalar_cannot_be_declared_over_nested_data(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    upsert(
        ctx,
        &name,
        vec![doc!("_id" => "a", "meta" => Value::r#struct([("tag", "x".into())]))],
    )
    .await;

    let err = update(ctx, &name, schema!("meta" => FieldSpec::text(false)), &[])
        .await
        .expect_err("`meta` holds a struct, not text");
    assert!(
        reason(err).contains("stored values are nested under `meta.tag`"),
        "the refusal has to name the nested column"
    );

    ctx.client
        .collection(&name)
        .upsert(vec![
            doc!("_id" => "b", "meta" => Value::r#struct([("tag", "y".into())])),
        ])
        .await
        .expect("the refused update left the collection accepting its own data");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_a_scalar_parent_cannot_have_declared_children(ctx: &mut ProjectTestContext) {
    let err = ctx
        .client
        .collections()
        .create(
            ctx.wrap("contradiction"),
            schema!("meta" => FieldSpec::text(false), "meta.tag" => FieldSpec::text(false)),
            None,
        )
        .await
        .expect_err("`meta` cannot be text and hold `meta.tag`");
    let Error::SchemaValidationError(errors) = err else {
        panic!("{err:?}")
    };
    assert!(errors.into_iter().any(|e| e
        == SchemaValidationError::NestedFieldUnderScalar {
            field: "meta.tag".to_string(),
            parent: "meta".to_string()
        }));

    let collection = create(ctx, schema!("meta.tag" => FieldSpec::text(false))).await;
    let err = update(
        ctx,
        &collection.name,
        schema!("meta" => FieldSpec::text(false)),
        &[],
    )
    .await
    .expect_err("an update cannot reach the same contradiction in two steps");
    assert!(matches!(err, Error::SchemaValidationError(_)), "{err:?}");
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_a_struct_path_has_no_empty_segments(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    for name in ["meta.", "a..b", ".a"] {
        let err = update(
            ctx,
            &collection.name,
            schema!(name => FieldSpec::text(false)),
            &[],
        )
        .await
        .unwrap_err();
        let Error::SchemaValidationError(errors) = err else {
            panic!("`{name}`: {err:?}")
        };
        assert!(
            errors
                .into_iter()
                .any(|e| e == SchemaValidationError::EmptyFieldName),
            "`{name}` names an empty segment"
        );
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_a_settled_index_stays_whole_through_the_next_build(ctx: &mut ProjectTestContext) {
    let collection = create(
        ctx,
        schema!("title" => FieldSpec::text(false), "summary" => FieldSpec::text(false)),
    )
    .await;
    let name = collection.name.clone();

    for batch in 0..6 {
        upsert(
            ctx,
            &name,
            (0..4)
                .map(|i| {
                    doc!(
                        "_id" => format!("d{batch}-{i}"),
                        "title" => "shared title term",
                        "summary" => "shared summary term"
                    )
                })
                .collect(),
        )
        .await;
    }
    let all: Vec<String> = (0..6)
        .flat_map(|b| (0..4).map(move |i| format!("d{b}-{i}")))
        .collect();
    let mut all = all;
    all.sort();

    let indexed = update(
        ctx,
        &name,
        schema!("title" => keyword(FieldSpec::text(false))),
        &[],
    )
    .await
    .expect("indexing title");
    assert_eq!(
        wait_for_index(async || {
            ids_matching(ctx, &name, "title", "title", None, Some(indexed.ssn)).await
        })
        .await,
        all
    );

    let updated = update(
        ctx,
        &name,
        schema!("summary" => keyword(FieldSpec::text(false))),
        &[],
    )
    .await
    .expect("indexing summary");

    let deadline = Instant::now() + Duration::from_secs(180);
    let mut served = 0;
    let mut refused_building = 0;
    loop {
        match ctx
            .client
            .collection(&name)
            .query(
                filter(r#match("title", Some("title"), None, false)).limit(100),
                None,
                Some(updated.ssn),
                None,
            )
            .await
        {
            Ok(docs) => {
                let mut ids: Vec<_> = docs
                    .iter()
                    .map(|doc| doc.id().unwrap().to_string())
                    .collect();
                ids.sort();
                assert_eq!(ids, all, "a settled index served a partial result");
                served += 1;
            }
            Err(err) => panic!("query on the settled field: {err:?}"),
        }
        match ctx
            .client
            .collection(&name)
            .query(
                filter(r#match("summary", Some("summary"), None, false)).limit(100),
                None,
                Some(updated.ssn),
                None,
            )
            .await
        {
            Ok(docs) => {
                let mut ids: Vec<_> = docs
                    .iter()
                    .map(|doc| doc.id().unwrap().to_string())
                    .collect();
                ids.sort();
                assert_eq!(ids, all, "the building index served a partial result");
                break;
            }
            Err(Error::IndexBuilding(_)) => refused_building += 1,
            Err(err) => panic!("query on the building field: {err:?}"),
        }
        assert!(Instant::now() < deadline, "summary never converged");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    assert!(
        served > 0,
        "the settled field was never actually queried while the second index built"
    );
    assert!(
        refused_building > 0,
        "the gate never refused a query on the field whose index was building"
    );
    println!("settled: {served} served; building: {refused_building} refused");
}

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
        wait_for_index(async || ids_matching(ctx, &name, "love", "summary", None, None).await).await,
        vec!["with"]
    );
}

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
            lsn.clone(),
            None,
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

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_reads_after_update(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, HashMap::new()).await;
    let name = collection.name.clone();
    let before = upsert(ctx, &name, docs()).await;
    for _ in 0..3 {
        let updated = update(
            ctx,
            &name,
            schema!("rating" => FieldSpec::integer(true)),
            &[],
        )
        .await
        .unwrap();
        assert_eq!(
            ctx.client
                .collection(&name)
                .count(Some(before.clone()), Some(updated.ssn), None)
                .await
                .unwrap(),
            3
        );
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
        .get(["pride", "late"], None, Some(lsn.clone()), None, None)
        .await
        .unwrap();
    assert_eq!(got.len(), 2);
    let found = ctx
        .client
        .collection(&name)
        .query(
            filter(field("rating").gte(3u32)).count(),
            Some(lsn),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(found, vec![doc!("_count" => 3u64)]);
}

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

#[test_context(ProjectTestContext)]
#[tokio::test]
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
    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    let found = ctx
        .client
        .collection(&name)
        .query(filter(field("rating").gte(1u32)).count(), None, None, None)
        .await
        .unwrap();
    assert_eq!(found, vec![doc!("_count" => 4u64)]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
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
    tokio::time::sleep(std::time::Duration::from_secs(12)).await;

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
                write_through_pause(async || {
                    client
                        .collection(&name)
                        .upsert(vec![doc!("_id" => format!("{w}-{i}"), "rating" => i)])
                        .await
                })
                .await;
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 50);
}

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
        .get(["pride", "gatsby"], None, Some(lsn.clone()), None, None)
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got["pride"]["title"], "Pride & Prejudice".into());
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_add_index_over_flushed_files(ctx: &mut ProjectTestContext) {
    let collection = create(ctx, schema!("title" => FieldSpec::text(true))).await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;

    // outlast the emulator's `flush_max_interval`, so the docs live in files and the index add
    // has something stale to rewrite
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    update(
        ctx,
        &name,
        schema!("summary" => keyword(FieldSpec::text(false))),
        &[],
    )
    .await
    .unwrap();

    // whatever the first successful answer is, it covers the files written before the index
    let ids =
        wait_for_index(async || ids_matching(ctx, &name, "love", "summary", None, None).await).await;
    assert_eq!(ids, vec!["gatsby".to_string(), "pride".to_string()]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_re_add_index_with_other_semantics(ctx: &mut ProjectTestContext) {
    let exact = FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Exact));
    let collection = create(
        ctx,
        schema!("title" => FieldSpec::text(true), "summary" => exact),
    )
    .await;
    let name = collection.name.clone();
    upsert(ctx, &name, docs()).await;

    // outlast the emulator's `flush_max_interval`, so the exact index lands in a file
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let ids = wait_for_index(async || {
        ids_matching(ctx, &name, "a whale and a captain", "summary", None, None).await
    })
    .await;
    assert_eq!(ids, vec!["moby".to_string()]);

    update(
        ctx,
        &name,
        schema!("summary" => FieldSpec::text(false)),
        &[],
    )
    .await
    .unwrap();
    let tokenized = update(
        ctx,
        &name,
        schema!("summary" => keyword(FieldSpec::text(false))),
        &[],
    )
    .await
    .unwrap();

    // tokenized now: a word out of the middle matches, which the verbatim index cannot answer
    assert_eq!(
        wait_for_index(async || {
            ids_matching(ctx, &name, "whale", "summary", None, Some(tokenized.ssn)).await
        })
        .await,
        vec!["moby".to_string()]
    );
}

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
            .get(
                ["seed", "0", "1", "2", "3", "4", "5"],
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(docs.len(), 7, "partition p{p}");
    }
}

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

#[test_context(ProjectTestContext)]
#[tokio::test]
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
                write_through_pause(async || {
                    client
                        .collection(&name)
                        .partition(&format!("fresh{p}"))
                        .upsert(vec![doc!("_id" => i.to_string(), "rating" => i)])
                        .await
                })
                .await;
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
            .get(["0", "1", "2", "3"], None, None, None, None)
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
            None,
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 20);
    for i in 0..20 {
        assert_eq!(docs[&i.to_string()]["title"], format!("edited{i}").into());
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
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
    // `a` carries a keyword index, so any update that declares it takes the layout path: the
    // candidate reserves an SSN, every partition is fenced, and files must converge before a
    // query on `a` is answered.
    fn indexed_text(required: bool) -> FieldSpec {
        keyword(FieldSpec::text(required))
    }
    // every field has a natural type; docs mostly hold it, sometimes another one. `m.*` are
    // nested, and a doc sometimes holds a scalar `m` instead.
    const FIELDS: [(&str, fn(bool) -> FieldSpec); 8] = [
        ("a", indexed_text),
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

        let (mut live, mut dead) = (vec![], vec![]);
        for (f, spec) in request {
            let old = model.schema.get(f);
            let gains_type = old.is_none();
            let gains_required = spec.required && !old.is_some_and(|o| o.required);
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
            0..=5 => {
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
            6 => {
                if let Some((f, old)) = model
                    .schema
                    .iter()
                    .filter(|(_, spec)| spec.index.is_some())
                    .choose(rng)
                {
                    schema.insert(
                        f.clone(),
                        FieldSpec {
                            index: None,
                            ..old.clone()
                        },
                    );
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
                    // the index goes with the old type: keeping it would be rejected for the
                    // index, not for the type change this arm is here to exercise
                    let mut spec = FieldSpec {
                        index: None,
                        ..model.schema[*f].clone()
                    };
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
    let mut accepted_writes = 0;
    let mut committed_updates = 0;
    let mut immediate_reads = 0;
    let mut dropped_indexes = 0;
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
                accepted_writes += usize::from(result.is_ok());
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
                accepted_writes += usize::from(result.is_ok());
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
                        committed_updates += 1;
                        dropped_indexes += model
                            .schema
                            .iter()
                            .filter(|(field, old)| {
                                old.index.is_some()
                                    && updated
                                        .schema
                                        .get(*field)
                                        .is_some_and(|new| new.index.is_none())
                            })
                            .count();
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
                let ssn = client.collections().get(&name).await.unwrap().ssn;
                for partition in PARTITIONS {
                    let ids = model.docs_in(partition);
                    if ids.is_empty() {
                        continue;
                    }
                    let got = (collection(partition)
                        .get(
                            ids.clone(),
                            None,
                            lsn.get(partition).cloned(),
                            Some(ssn),
                            None,
                        )
                        .await)
                        .unwrap();
                    println!("{step}: read {} docs from {partition:?}", ids.len());
                    immediate_reads += 1;
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
    println!("accepted_writes={accepted_writes} committed_updates={committed_updates} dropped_indexes={dropped_indexes} immediate_reads={immediate_reads}");
    assert!(accepted_writes > 0 && committed_updates > 0 && immediate_reads > 0);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_ssn_advances_with_every_committed_change(ctx: &mut ProjectTestContext) {
    let created = create(ctx, schema!("title" => FieldSpec::text(false))).await;
    assert_eq!(created.ssn, 1);

    let updated = update(
        ctx,
        &ctx.wrap("books"),
        schema!("summary" => FieldSpec::text(false)),
        &[],
    )
    .await
    .expect("adding an optional field only loosens");
    assert!(
        updated.ssn > created.ssn,
        "{} did not advance past {}",
        updated.ssn,
        created.ssn
    );

    let fetched = ctx
        .client
        .collections()
        .get(&ctx.wrap("books"))
        .await
        .expect("could not get collection");
    assert_eq!(fetched.ssn, updated.ssn);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_required_ssn_plans_under_the_committed_schema(ctx: &mut ProjectTestContext) {
    create(ctx, schema!("title" => keyword(FieldSpec::text(false)))).await;
    let lsn = upsert(ctx, &ctx.wrap("books"), docs()).await;

    let updated = update(
        ctx,
        &ctx.wrap("books"),
        schema!("summary" => keyword(FieldSpec::text(false))),
        &[],
    )
    .await
    .expect("adding an indexed field");

    // the floor is met before the reader's own cache would have expired
    let ids = wait_for_index(async || {
        ctx.client
            .collection(&ctx.wrap("books"))
            .query(
                filter(r#match("love", Some("summary"), None, false))
                    .select([("title", field("title"))])
                    .limit(100),
                Some(lsn.clone()),
                Some(updated.ssn),
                None,
            )
            .await
    })
    .await;
    let mut ids: Vec<String> = ids.iter().map(|d| d.id().unwrap().to_string()).collect();
    ids.sort();
    assert_eq!(ids, vec!["gatsby", "pride"]);
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_an_ssn_that_never_commits_is_refused(ctx: &mut ProjectTestContext) {
    let created = create(ctx, schema!("title" => FieldSpec::text(false))).await;

    let error = ctx
        .client
        .collection(&ctx.wrap("books"))
        .count(None, Some(created.ssn + 1), None)
        .await
        .expect_err("a version nothing committed cannot be waited for");
    match error {
        Error::SsnUnavailable(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test_context(ProjectTestContext)]
#[tokio::test]
async fn test_add_vector_index_over_flushed_vectors(ctx: &mut ProjectTestContext) {
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

    // outlast the emulator's `flush_max_interval`, so the vectors live in files that the index
    // add has no reason to rewrite
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    update(
        ctx,
        &name,
        schema!("vector" => FieldSpec::f32_vector(2, false)
            .with_index(FieldIndex::vector(VectorDistanceMetric::Euclidean))),
        &[],
    )
    .await
    .expect("update failed");

    assert_eq!(
        wait_for_index(async || nearest(ctx, &name).await).await,
        vec!["a", "c"]
    );
}
