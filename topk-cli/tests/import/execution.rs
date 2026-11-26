use futures::TryStreamExt;
use prost::Message;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::plan::{Plan, Target};
use topk::import::source::{Cursor, Resume};
use topk::import::state::{Progress, Run};
use topk::import::{Error, Event, Import, WriteOptions};
use topk_rs::proto::v1::control::{FieldIndex, FieldSpec, KeywordIndexType, VectorDistanceMetric};
use topk_rs::Error as SdkError;

use topk_rs::proto::v1::data::{stage::sort_stage::SortOrder, ConsistencyLevel};
use topk_rs::query::{field, fns, select};

use crate::import::common::{source, target, Ctx, AIMD};

#[rstest_ctx(Ctx)]
async fn prepares_then_executes(ctx: &Ctx) {
    let collection = ctx.collection("execution");
    let from = ctx.cli.file("rows.csv", "id,title\n1,one\n2,two\n3,three");
    let source = source(&from).await.unwrap();
    let target = target(&from, "id", [("title", FieldSpec::text(false))]).capped(3);
    let mut plan = Plan {
        collection: collection.clone(),
        target,
    };
    let create = plan
        .resolve(&source, &ctx.client, &[], &[], &[])
        .await
        .unwrap();
    let directory = ctx.cli.path("state");
    let run = Run::start(
        &directory,
        ctx.state(
            &source,
            plan,
            WriteOptions {
                continue_on_error: false,
                concurrency: Some(2),
                batch_bytes: 32,
            },
        ),
    )
    .await
    .unwrap();
    let path = directory.join(format!("{}.toml", run.id));
    let import = Import::start(source, ctx.client.clone(), run, create)
        .await
        .unwrap();
    assert!(!ctx.exists(&collection).await);
    let events: Vec<_> = import.execute(AIMD).try_collect().await.unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Completed { .. }))
            .count(),
        1
    );
    for event in events {
        match event {
            Event::Completed { outcome } => {
                assert_eq!((outcome.rows, outcome.failed), (3, 0));
            }
            Event::Progress { written, .. } => assert!(written <= 3),
            Event::Skipped { error, .. } => panic!("{error}"),
        }
    }
    assert!(!path.exists());
    assert_eq!(
        ctx.docs(&collection, [1, 2, 3]).await,
        [
            json!({"_id": "1", "title": "one"}),
            json!({"_id": "2", "title": "two"}),
            json!({"_id": "3", "title": "three"}),
        ]
    );
}

#[rstest_ctx(Ctx)]
async fn resumes_saved_prefix(ctx: &Ctx) {
    let collection = ctx.collection("resume");
    let from = ctx
        .cli
        .file("rows.csv", "id,title\n1,one\n2,two\n3,three\n4,four");
    let source = source(&from).await.unwrap();
    let target = target(&from, "id", [("title", FieldSpec::text(false))]).capped(3);
    let plan = Plan {
        collection: collection.clone(),
        target,
    };
    let mut state = ctx.state(
        &source,
        plan,
        WriteOptions {
            continue_on_error: false,
            concurrency: Some(2),
            batch_bytes: 32,
        },
    );
    state.progress = Some(Progress::After(Resume {
        open: vec![Cursor::Offset {
            part: from,
            rows: 1,
        }],
        done: Vec::new(),
        consumed: 1,
    }));
    let directory = ctx.cli.path("state");
    let run = Run::start(&directory, state).await.unwrap();
    run.save().unwrap();
    let id = run.id;
    drop(run);
    let run = Run::load(&directory, id).await.unwrap();
    let import = Import::resume(source, ctx.client.clone(), run)
        .await
        .unwrap();
    let events: Vec<_> = import.execute(AIMD).try_collect().await.unwrap();
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                Event::Completed { outcome } => Some((outcome.rows, outcome.failed)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [(2, 0)]
    );
    assert!(events.iter().all(|event| matches!(
        event,
        Event::Completed { .. } | Event::Progress { written: 0..=2, .. }
    )));
    assert!(!directory.join(format!("{id}.toml")).exists());
    assert_eq!(
        ctx.docs(&collection, [1, 2, 3, 4]).await,
        [
            json!({"_id": "2", "title": "two"}),
            json!({"_id": "3", "title": "three"})
        ]
    );
}

#[rstest_ctx(Ctx)]
#[case::recover_batch(true, 2, 1, 1_000_000)]
#[case::reject_batch(false, 2, 1, 1_000_000)]
#[case::all_rejected(true, 0, 3, 1_000_000)]
#[case::rejected_singleton(true, 0, 1, 1)]
async fn backend_rejections(
    ctx: &Ctx,
    #[case] continue_on_error: bool,
    #[case] valid: usize,
    #[case] invalid: usize,
    #[case] batch_bytes: u64,
) {
    let collection = ctx.collection("backend-rejections");
    let docs: Vec<_> = (0..invalid + valid)
        .map(|i| topk_rs::doc!("_id" => i.to_string(), "tag" => if i < invalid { "x".repeat(10_000) } else { "ok".into() }))
        .collect();
    let expected_bytes: u64 = docs
        .iter()
        .skip(invalid)
        .map(|doc| doc.encoded_len() as u64)
        .sum();
    let from = ctx.cli.seed_parquet("rows", docs);
    let source = source(&from).await.unwrap();
    let target = target(
        &from,
        "_id",
        [(
            "tag",
            FieldSpec::text(false).with_index(FieldIndex::keyword(KeywordIndexType::Exact)),
        )],
    );
    let mut plan = Plan {
        collection: collection.clone(),
        target,
    };
    let create = plan
        .resolve(&source, &ctx.client, &[], &[], &[])
        .await
        .unwrap();
    let directory = ctx.cli.path("state");
    let run = Run::start(
        &directory,
        ctx.state(
            &source,
            plan,
            WriteOptions {
                continue_on_error,
                concurrency: Some(2),
                batch_bytes,
            },
        ),
    )
    .await
    .unwrap();
    let id = run.id;
    let import = Import::start(source, ctx.client.clone(), run, create)
        .await
        .unwrap();
    import.run().save().unwrap();
    let result: Result<Vec<_>, _> = import.execute(AIMD).try_collect().await;
    assert_eq!(result.is_ok(), continue_on_error);
    match result {
        Ok(events) => {
            for event in &events {
                match event {
                    Event::Progress { bytes, .. } => assert!(*bytes <= expected_bytes),
                    Event::Completed { outcome, .. } => assert_eq!(outcome.bytes, expected_bytes),
                    _ => {}
                }
            }
            let skipped: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Skipped { error, .. } => Some(error),
                    _ => None,
                })
                .collect();
            assert_eq!(skipped.len(), invalid);
            for error in skipped {
                assert!(matches!(
                    error,
                    Error::Topk(SdkError::DocumentValidationError(_))
                ));
            }
            assert_eq!(
                events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Completed { outcome } => Some((outcome.rows, outcome.failed)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                [(valid, invalid)]
            );
        }
        Err(error) => assert!(matches!(
            error,
            Error::Topk(SdkError::DocumentValidationError(_))
        )),
    }
    let expected: Vec<_> = (invalid..invalid + valid)
        .filter(|_| continue_on_error)
        .map(|i| json!({"_id": i.to_string(), "tag": "ok"}))
        .collect();
    assert_eq!(ctx.docs(&collection, 0..invalid + valid).await, expected);
    if continue_on_error {
        assert!(!directory.join(format!("{id}.toml")).exists());
    } else {
        let run = Run::load(&directory, id).await.unwrap();
        assert!(run.state().progress.is_none());
    }
}

#[rstest_ctx(Ctx)]
#[case::matching_schema(false)]
#[case::conflicting_schema(true)]
async fn collection_created_after_plan(ctx: &Ctx, #[case] conflict: bool) {
    let collection = ctx.collection("create-race");
    let from = ctx.cli.file("rows.csv", "id,title\n1,one");
    let source = source(&from).await.unwrap();
    let mut plan = Plan {
        collection: collection.clone(),
        target: Target::new(source.discover().await.unwrap(), None).unwrap(),
    };
    let create = plan
        .resolve(&source, &ctx.client, &[], &[], &[])
        .await
        .unwrap();
    let run = Run::start(
        &ctx.cli.path("state"),
        ctx.state(
            &source,
            plan,
            WriteOptions {
                continue_on_error: false,
                concurrency: Some(1),
                batch_bytes: 1024,
            },
        ),
    )
    .await
    .unwrap();
    let import = Import::start(source, ctx.client.clone(), run, create)
        .await
        .unwrap();
    ctx.client
        .collections()
        .create(
            &collection,
            [(
                "title".into(),
                if conflict {
                    FieldSpec::integer(false)
                } else {
                    FieldSpec::text(false)
                },
            )],
            None,
        )
        .await
        .unwrap();
    let result: Result<Vec<_>, _> = import.execute(AIMD).try_collect().await;
    if conflict {
        assert!(matches!(result, Err(Error::SchemaMismatch(_))));
        assert!(ctx.docs(&collection, ["1"]).await.is_empty());
    } else {
        result.unwrap();
        assert_eq!(
            ctx.docs(&collection, ["1"]).await,
            [json!({"_id":"1","title":"one"})]
        );
    }
}

#[rstest_ctx(Ctx)]
#[case::direct_import(false)]
#[case::resumed_import(true)]
async fn inferred_vector_uses_destination(ctx: &Ctx, #[case] resume: bool) {
    let collection = ctx.collection("resolved-vector");
    ctx.client
        .collections()
        .create(
            &collection,
            [(
                "embedding".into(),
                FieldSpec::f32_vector(3, true)
                    .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine)),
            )],
            None,
        )
        .await
        .unwrap();
    let from = ctx.cli.parquet(
        "vectors.parquet",
        if resume {
            "SELECT 'one' AS id, [1.0, 0.0]::FLOAT[] AS embedding"
        } else {
            "SELECT 'one' AS id, [1.0, 0.0, 0.0]::FLOAT[] AS embedding"
        },
    );
    let input = source(&from).await.unwrap();
    let mut plan = Plan {
        collection: collection.clone(),
        target: Target::new(input.discover().await.unwrap(), None).unwrap(),
    };
    let create = plan
        .resolve(&input, &ctx.client, &[], &[], &[])
        .await
        .unwrap();
    let directory = ctx.cli.path("state");
    let run = Run::start(
        &directory,
        ctx.state(
            &input,
            plan,
            WriteOptions {
                continue_on_error: false,
                concurrency: Some(1),
                batch_bytes: 1024,
            },
        ),
    )
    .await
    .unwrap();
    let id = run.id;
    let mut import = Import::start(input, ctx.client.clone(), run, create)
        .await
        .unwrap();
    let destination = ctx.client.collections().get(&collection).await.unwrap();
    assert_eq!(
        import.run().state().plan.target.fields["embedding"].schema,
        destination.schema["embedding"]
    );
    import.run().save().unwrap();
    if resume {
        let failed = import.execute(AIMD).try_collect::<Vec<_>>().await;
        assert!(failed.err().unwrap().to_string().contains("embedding"));
        ctx.cli.parquet(
            "vectors.parquet",
            "SELECT 'one' AS id, [1.0, 0.0, 0.0]::FLOAT[] AS embedding",
        );
        let run = Run::load(&directory, id).await.unwrap();
        assert_eq!(
            run.state().plan.target.fields["embedding"].schema,
            destination.schema["embedding"]
        );
        import = Import::resume(source(&from).await.unwrap(), ctx.client.clone(), run)
            .await
            .unwrap();
    }
    import.execute(AIMD).try_collect::<Vec<_>>().await.unwrap();
    assert_eq!(
        ctx.docs(&collection, ["one"]).await,
        [json!({"_id":"one","embedding":[1.0,0.0,0.0]})]
    );
    let docs = ctx
        .client
        .collection(&collection)
        .query(
            select([(
                "distance",
                fns::vector_distance("embedding", vec![1.0f32, 0.0, 0.0]),
            )])
            .sort((field("distance"), SortOrder::Asc))
            .limit(1),
            None,
            Some(ConsistencyLevel::Strong),
        )
        .await
        .unwrap();
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].fields["_id"].as_string(), Some("one"));
}
