use crate::import::common::*;
use test_context::test_context;
use test_macros::rstest_ctx;
use topk::import::{Checkpoint, Cursor, Destination, Mark, Run, Spec, State};

fn saved_spec(a: &str, b: &str, c: &str) -> String {
    format!(
        "[a]\nfrom = \"a.parquet\"\nid = \"_id\"\n{a}\n[a.fields]\ntitle = {{ type = \"text\" }}\n\n\
         [b]\nfrom = \"b.parquet\"\nid = \"_id\"\n{b}\n[b.fields]\ntitle = {{ type = \"text\" }}\n\n\
         [c]\nfrom = \"c.parquet\"\nid = \"_id\"\n{c}\n[c.fields]\ntitle = {{ type = \"text\" }}\n"
    )
}

fn saved_destination(region: &str) -> Destination {
    Destination {
        project_id: "p1".parse().unwrap(),
        region: region.into(),
    }
}

#[test]
fn an_unchanged_run_keeps_its_checkpoints() {
    let plan: Spec = toml::from_str(&saved_spec("", "", "")).unwrap();
    let mut state = State::start("books.parquet", saved_destination("dev"), plan).unwrap();
    let cursor = Checkpoint {
        cursor: Cursor::Key("200".to_string()),
        consumed: 20,
    };
    state
        .cursors
        .insert("b".to_string(), Mark::After(cursor.clone()));
    state.cursors.insert("c".to_string(), Mark::Done);
    assert!(matches!(state.cursors.get("b"), Some(Mark::After(actual)) if *actual == cursor));
    assert!(matches!(state.cursors.get("c"), Some(Mark::Done)));
    assert_eq!(
        state
            .pending()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test_context(Scratch)]
#[tokio::test]
async fn a_run_refuses_a_different_source(ctx: &mut Scratch) {
    let stored = saved_spec("", "", "");
    let plan: Spec = toml::from_str(&stored).expect("spec parses");
    let state = State::start("books.parquet", saved_destination("dev"), plan).unwrap();
    ctx.save(&state);
    let message = ctx
        .topk()
        .args(["import", "other.parquet", "--resume", &state.id.to_string()])
        .fails();
    assert!(message.contains("books.parquet"), "got: {message}");
}

#[test_context(Scratch)]
#[tokio::test]
async fn checkpoint_past_limit_is_rejected(ctx: &mut Scratch) {
    let plan: Spec = toml::from_str(&saved_spec("limit = 5", "", "")).unwrap();
    let mut state = State::start("books.parquet", saved_destination("dev"), plan).unwrap();
    state.cursors.insert(
        "a".to_string(),
        Mark::After(Checkpoint {
            cursor: Cursor::Key("6".to_string()),
            consumed: 6,
        }),
    );
    ctx.save(&state);
    let error = ctx
        .topk()
        .args(["import", "--resume", &state.id.to_string()])
        .fails();
    assert!(
        error.contains("checkpoint consumed 6 rows, exceeding limit 5"),
        "{error}"
    );
}

#[rstest_ctx(Scratch)]
#[case::key(Cursor::Key("42".into()))]
#[case::offset(Cursor::Offset { part: "part.parquet".into(), rows: 42 })]
#[case::page(Cursor::Page { pit: "pit".into(), sort: vec![42] })]
async fn saved_plan_round_trips(ctx: &mut Scratch, #[case] cursor: Cursor) {
    std::env::set_var("TOPK_IMPORT_STATE_DIR", state_dir());
    let spec: Spec = toml::from_str(&saved_spec("", "", "")).unwrap();
    let source = "s3://bucket/*.parquet";
    let state = State::start(source, saved_destination("saved"), spec.clone()).unwrap();
    let expected = state.spec.clone();
    let id = ctx.save_stopped(state, "a", cursor.clone(), 42);
    let run = Run::load(id).await.unwrap();
    assert_eq!(run.source, source);
    assert_eq!(run.destination.project_id.as_str(), "p1");
    assert_eq!(run.destination.region, "saved");
    assert!(run.spec.collections == expected.collections);
    assert!(
        matches!(&run.cursors["a"], Mark::After(checkpoint) if checkpoint.cursor == cursor && checkpoint.consumed == 42)
    );
    run.remove();
}
