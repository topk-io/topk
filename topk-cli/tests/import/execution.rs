use futures::TryStreamExt;
use serde_json::json;
use test_macros::rstest_ctx;
use topk::import::source::{Checkpoint, Cursor};
use topk::import::spec::Spec;
use topk::import::state::{Progress, Run};
use topk::import::{Error, Event, Import, WriteOptions};
use topk_rs::proto::v1::control::{FieldIndex, FieldSpec, KeywordIndexType};

use crate::import::common::{source, target, Ctx};

#[rstest_ctx(Ctx)]
async fn prepares_then_executes(ctx: &Ctx) {
    let collection = ctx.collection("execution");
    let from = ctx.cli.file("rows.csv", "id,title\n1,one\n2,two\n3,three");
    let source = source(&from).await.unwrap();
    let target = target(&from, "id", [("title", FieldSpec::text(false))]).capped(3);
    let spec = Spec::try_from(indexmap::IndexMap::from([(collection.clone(), target)])).unwrap();
    let directory = ctx.cli.path("state");
    let run = Run::start(&directory, ctx.state(&source, spec))
        .await
        .unwrap();
    let path = directory.join(format!("{}.toml", run.id));
    let import = Import::prepare(source, (*ctx.client).clone(), run)
        .await
        .unwrap();
    assert!(!ctx.exists(&collection).await);
    let events: Vec<_> = import
        .execute(WriteOptions {
            continue_on_error: false,
            concurrency: 2,
            batch_bytes: bytesize::ByteSize(32),
        })
        .try_collect()
        .await
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Completed { .. }))
            .count(),
        1
    );
    for event in events {
        match event {
            Event::Completed {
                collection: name,
                outcome,
            } => {
                assert_eq!(name, collection);
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
    let spec = Spec::try_from(indexmap::IndexMap::from([(collection.clone(), target)])).unwrap();
    let mut state = ctx.state(&source, spec);
    state.progress.insert(
        collection.clone(),
        Progress::After(Checkpoint {
            cursor: Cursor::Offset {
                part: from,
                rows: 1,
            },
            consumed: 1,
        }),
    );
    let directory = ctx.cli.path("state");
    let run = Run::start(&directory, state).await.unwrap();
    run.save().unwrap();
    let id = run.id;
    drop(run);
    let run = Run::load(&directory, id).await.unwrap();
    let import = Import::prepare(source, (*ctx.client).clone(), run)
        .await
        .unwrap();
    let events: Vec<_> = import
        .execute(WriteOptions {
            continue_on_error: false,
            concurrency: 2,
            batch_bytes: bytesize::ByteSize(32),
        })
        .try_collect()
        .await
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                Event::Completed {
                    collection,
                    outcome,
                } => Some((collection.as_str(), outcome.rows, outcome.failed)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [(collection.as_str(), 2, 0)]
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
    let spec = Spec::try_from(indexmap::IndexMap::from([(collection.clone(), target)])).unwrap();
    let directory = ctx.cli.path("state");
    let run = Run::start(&directory, ctx.state(&source, spec))
        .await
        .unwrap();
    let id = run.id;
    let import = Import::prepare(source, (*ctx.client).clone(), run)
        .await
        .unwrap();
    let result: Result<Vec<_>, _> = import
        .execute(WriteOptions {
            continue_on_error,
            concurrency: 2,
            batch_bytes: bytesize::ByteSize(batch_bytes),
        })
        .try_collect()
        .await;
    assert_eq!(result.is_ok(), continue_on_error);
    match result {
        Ok(events) => {
            let skipped: Vec<_> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Skipped { collection, error } => Some((collection, error)),
                    _ => None,
                })
                .collect();
            assert_eq!(skipped.len(), invalid);
            for (name, error) in skipped {
                assert_eq!(name, &collection);
                assert!(matches!(error, Error::RejectedDocuments(_)));
            }
            assert_eq!(
                events
                    .iter()
                    .filter_map(|event| match event {
                        Event::Completed {
                            collection,
                            outcome,
                        } => Some((collection.as_str(), outcome.rows, outcome.failed)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                [(collection.as_str(), valid, invalid)]
            );
        }
        Err(error) => assert!(matches!(error, Error::RejectedDocuments(_))),
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
        assert!(run.state().progress.is_empty());
    }
}
