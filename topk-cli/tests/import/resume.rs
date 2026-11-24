use test_macros::rstest_ctx;
use topk::import::plan::Plan;
use topk::import::source::{Cursor, Resume};
use topk::import::state::{Progress, Run, State};
use topk::import::WriteOptions;
use topk::scope::Scope;
use topk_rs::proto::v1::control::{FieldIndex, FieldSpec, VectorDistanceMetric};

use crate::import::common::{refused, target, Scratch};

fn state() -> State {
    let plan = Plan {
        collection: "a".into(),
        target: target(
            "a.parquet",
            "_id",
            [
                ("title", FieldSpec::text(false)),
                (
                    "embedding",
                    FieldSpec::f32_vector(3, false)
                        .with_index(FieldIndex::vector(VectorDistanceMetric::Cosine)),
                ),
            ],
        ),
    };
    State::new(
        "books.parquet".into(),
        Scope {
            project_id: "p1".parse().unwrap(),
            region: "dev".into(),
        },
        plan,
        WriteOptions {
            continue_on_error: false,
            concurrency: 16,
            batch_bytes: 1_048_576,
        },
    )
}

#[test]
fn done_after_last_checkpoint() {
    let mut state = state();
    assert!(!state.done());
    state.progress = Some(Progress::After(Resume {
        open: vec![Cursor::Key("200".into())],
        done: Vec::new(),
        consumed: 20,
    }));
    assert!(!state.done());
    state.plan.target.limit = Some(20);
    assert!(state.done());
    state.plan.target.limit = None;
    state.progress = Some(Progress::Done);
    assert!(state.done());
}

#[test]
fn different_source_rejected() {
    let error = refused(state().source(Some(&"other.parquet".parse().unwrap())));
    assert!(error.contains("books.parquet"), "{error}");
}

#[test]
fn checkpoint_past_limit() {
    let mut state = state();
    state.plan.target.limit = Some(5);
    state.progress = Some(Progress::After(Resume {
        open: vec![Cursor::Key("6".into())],
        done: Vec::new(),
        consumed: 6,
    }));
    let error = refused(state.validate());
    assert!(
        error.contains("checkpoint consumed 6 rows, exceeding limit 5"),
        "{error}"
    );
}

#[rstest_ctx(Scratch)]
async fn lock_needs_a_state_file(ctx: &Scratch) {
    let directory = ctx.path("state");
    let run = Run::start(&directory, state()).await.unwrap();
    let lock = directory.join(format!("{}.lock", run.id));
    assert!(lock.exists());
    drop(run);
    assert!(!lock.exists());
    assert!(Run::load(&directory, "1234abcd".parse().unwrap())
        .await
        .is_err());
    assert!(!directory.join("1234abcd.lock").exists());
}

#[rstest_ctx(Scratch)]
#[case::key(Cursor::Key("42".into()))]
#[case::sql_key(Cursor::Sql { column: "source_id".into(), value: "42".into() })]
#[case::mongo_key(Cursor::Mongo(r#"{"$numberInt":"42"}"#.into()))]
#[case::offset(Cursor::Offset { part: "part.parquet".into(), rows: 42 })]
#[case::page(Cursor::Page { pit: "pit".into(), sort: vec![42] })]
async fn saved_plan_round_trips(ctx: &Scratch, #[case] cursor: Cursor) {
    let directory = ctx.path("state");
    let mut saved = state();
    saved.progress = Some(Progress::After(Resume {
        open: vec![cursor.clone()],
        done: Vec::new(),
        consumed: 42,
    }));
    let run = Run::start(&directory, saved).await.unwrap();
    run.save().unwrap();
    let id = run.id;
    assert!(directory.join(format!("{id}.toml")).exists());
    drop(run);
    let run = Run::load(&directory, id).await.unwrap();
    assert_eq!(run.state().source, "books.parquet");
    assert_eq!(run.state().destination.project_id.as_str(), "p1");
    assert_eq!(run.state().destination.region, "dev");
    assert!(run.state().plan == state().plan);
    assert!(
        matches!(&run.state().progress, Some(Progress::After(resume)) if resume.open == [cursor.clone()] && resume.consumed == 42)
    );
    run.remove().unwrap();
}

#[rstest_ctx(Scratch)]
fn equivalent_file_paths_resume(ctx: &Scratch) {
    let mut state = state();
    state.source = ctx.file("rows.jsonl", "");
    let equivalent = ctx.path("./rows.jsonl").display().to_string();
    assert_eq!(
        state
            .source(Some(&equivalent.parse().unwrap()))
            .unwrap()
            .to_string(),
        state.source
    );
}

#[test]
fn saved_file_scheme_resumes() {
    let mut state = state();
    state.source = "file:///data/part_*.parquet".into();
    let supplied = "file:///data/part_*.parquet".parse().unwrap();
    assert_eq!(
        state.source(Some(&supplied)).unwrap().to_string(),
        "/data/part_*.parquet"
    );
}

#[test]
fn credentials_preserve_selector() {
    let mut state = state();
    state.source = "postgres://u:***@host/db?sslmode=require#public.orders".into();
    let supplied = "postgres://u:new-password@host/db?sslmode=require#public.orders"
        .parse()
        .unwrap();
    assert_eq!(
        state.source(Some(&supplied)).unwrap().password(),
        Some("new-password")
    );
    assert!(refused(state.source(None)).contains("needs source credentials"));
    let changed = "postgres://u:new-password@host/db?sslmode=require#public.customers"
        .parse()
        .unwrap();
    assert!(refused(state.source(Some(&changed))).contains("saved run reads"));
    state.source = "postgres://u:***@host/db".into();
    assert!(refused(state.source(None)).contains("old saved runs need a new import"));
}
