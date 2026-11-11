use test_macros::rstest_ctx;
use topk::import::source::{Checkpoint, Cursor};
use topk::import::spec::Spec;
use topk::import::state::{DestinationId, Progress, Run, State};
use topk_rs::proto::v1::control::FieldSpec;

use crate::import::common::{refused, target, Scratch};

fn state() -> State {
    let spec = Spec::try_from(
        ["a", "b", "c"]
            .into_iter()
            .map(|name| {
                (
                    name.to_string(),
                    target(
                        &format!("{name}.parquet"),
                        "_id",
                        [("title", FieldSpec::text(false))],
                    ),
                )
            })
            .collect::<indexmap::IndexMap<_, _>>(),
    )
    .unwrap();
    State::new(
        "books.parquet".into(),
        DestinationId {
            project_id: "p1".parse().unwrap(),
            region: "dev".into(),
        },
        spec,
    )
}

#[test]
fn pending_collections() {
    let mut state = state();
    state.progress.insert(
        "b".into(),
        Progress::After(Checkpoint {
            cursor: Cursor::Key("200".into()),
            consumed: 20,
        }),
    );
    state.progress.insert("c".into(), Progress::Done);
    assert_eq!(
        state
            .pending()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    state.spec.collections["b"].limit = Some(20);
    assert_eq!(
        state
            .pending()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["a"]
    );
}

#[test]
fn different_source_rejected() {
    let error = refused(state().source(Some(&"other.parquet".parse().unwrap())));
    assert!(error.contains("books.parquet"), "{error}");
}

#[test]
fn checkpoint_past_limit() {
    let mut state = state();
    state.spec.collections["a"].limit = Some(5);
    state.progress.insert(
        "a".into(),
        Progress::After(Checkpoint {
            cursor: Cursor::Key("6".into()),
            consumed: 6,
        }),
    );
    let error = refused(state.validate());
    assert!(
        error.contains("checkpoint consumed 6 rows, exceeding limit 5"),
        "{error}"
    );
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
    saved.progress.insert(
        "a".into(),
        Progress::After(Checkpoint {
            cursor: cursor.clone(),
            consumed: 42,
        }),
    );
    let run = Run::start(&directory, saved).await.unwrap();
    run.save().unwrap();
    let id = run.id;
    drop(run);
    let run = Run::load(&directory, id).await.unwrap();
    assert_eq!(run.state().source, "books.parquet");
    assert_eq!(run.state().destination.project_id.as_str(), "p1");
    assert_eq!(run.state().destination.region, "dev");
    assert!(run.state().spec.collections == state().spec.collections);
    assert!(
        matches!(&run.state().progress["a"], Progress::After(checkpoint) if checkpoint.cursor == cursor && checkpoint.consumed == 42)
    );
    run.remove().unwrap();
}

#[test]
fn legacy_shapes_in_saved_state() {
    let state: State = toml::from_str(
        r#"
source = "rows.parquet"
project_id = "p1"
region = "dev"
[spec.rows]
from = "rows.parquet"
id = "_id"
[spec.rows.fields]
dense = { type = "f32_vector", dim = 3 }
sparse = { type = "f16_sparse_vector" }
matrix = { type = "i8_matrix", cols = 8 }
[cursors.rows.after]
key = "last"
consumed = 2
"#,
    )
    .unwrap();
    let encoded = toml::to_string(&state).unwrap();
    assert!(encoded.contains("[cursors.rows.after]"));
    assert!(!encoded.contains("[progress"));
    let restored: State = toml::from_str(&encoded).unwrap();
    assert!(state.spec.collections == restored.spec.collections);
    assert!(matches!(&restored.progress["rows"], Progress::After(c) if c.consumed == 2));
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
