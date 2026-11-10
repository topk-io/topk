use std::collections::BTreeMap;

use crate::import::common::*;
use topk::import::{Checkpoint, Cursor, Mark, Spec, State};
use topk::ProjectId;

fn project(id: &str) -> ProjectId {
    id.parse().unwrap()
}

fn spec(a: &str, b: &str, c: &str) -> String {
    format!(
        "[a]\nfrom = \"a.parquet\"\nid = \"_id\"\n{a}\n[a.fields]\ntitle = {{ type = \"text\" }}\n\n\
         [b]\nfrom = \"b.parquet\"\nid = \"_id\"\n{b}\n[b.fields]\ntitle = {{ type = \"text\" }}\n\n\
         [c]\nfrom = \"c.parquet\"\nid = \"_id\"\n{c}\n[c.fields]\ntitle = {{ type = \"text\" }}\n"
    )
}

/// An edited spec invalidates that collection's cursor alone: the others resume
/// where they stopped, and a collection already done drops out of the run.
#[test]
fn an_edited_target_starts_over_without_disturbing_the_others() {
    let stored = spec("", "", "");
    let plan: Spec = toml::from_str(&stored).unwrap();
    let mut state = State::new("books.parquet", project("p1"), &plan).unwrap();
    state.cursors.insert(
        "a".to_string(),
        Mark::After(Checkpoint {
            cursor: Cursor::Key("100".to_string()),
            consumed: 10,
        }),
    );
    state.cursors.insert(
        "b".to_string(),
        Mark::After(Checkpoint {
            cursor: Cursor::Key("200".to_string()),
            consumed: 20,
        }),
    );
    state.cursors.insert("c".to_string(), Mark::Done);

    let edited = spec("limit = 5", "", "");
    let mut plan: Spec = toml::from_str(&edited).expect("spec parses");
    let (done, after) = state
        .resume("books.parquet", &project("p1"), &mut plan)
        .expect("same source reconciles");

    assert_eq!(done, 1, "c was already imported");
    assert_eq!(
        toml::from_str::<Spec>(&state.spec)
            .unwrap()
            .collections
            .len(),
        3
    );
    assert_eq!(
        after,
        BTreeMap::from([(
            "b".to_string(),
            Checkpoint {
                cursor: Cursor::Key("200".to_string()),
                consumed: 20
            }
        )]),
        "a lost its cursor, b kept it"
    );
    assert_eq!(
        plan.collections.keys().collect::<Vec<_>>(),
        ["a", "b"],
        "c drops out of the run"
    );
}

#[test]
fn a_run_refuses_a_different_source() {
    let stored = spec("", "", "");
    let mut plan: Spec = toml::from_str(&stored).expect("spec parses");
    let mut state = State::new("books.parquet", project("p1"), &plan).unwrap();
    let message = refused(state.resume("other.parquet", &project("p1"), &mut plan));
    assert!(message.contains("books.parquet"), "got: {message}");
}

#[test]
fn cursors_round_trip_in_run_state() {
    let plan: Spec = toml::from_str(&spec("", "", "")).unwrap();
    let mut state = State::new("books.parquet", project("p1"), &plan).unwrap();
    assert!(toml::from_str::<Spec>(&state.spec).unwrap().collections == plan.collections);
    let cursors = [
        ("a", Cursor::Key("42".to_string())),
        (
            "b",
            Cursor::Offset {
                part: "b.parquet".to_string(),
                rows: 300,
            },
        ),
        (
            "c",
            Cursor::Page {
                pit: "opaque".to_string(),
                sort: vec![7],
            },
        ),
    ];
    for (name, cursor) in &cursors {
        state.cursors.insert(
            (*name).to_string(),
            Mark::After(Checkpoint {
                cursor: cursor.clone(),
                consumed: 300,
            }),
        );
    }

    let encoded = toml::to_string_pretty(&state).expect("state serializes");
    let decoded: State = toml::from_str(&encoded).expect("state deserializes");
    assert_eq!(decoded.project_id, project("p1"), "{encoded}");
    for (name, expected) in cursors {
        let Some(Mark::After(actual)) = decoded.cursors.get(name) else {
            panic!("missing cursor {name:?} in {encoded}");
        };
        assert_eq!(actual.cursor, expected, "{encoded}");
        assert_eq!(actual.consumed, 300);
    }
}

/// A resumed run keeps the project it was started for.
#[test]
fn a_resume_cannot_switch_projects() {
    let mut plan: Spec = toml::from_str(&spec("", "", "")).expect("spec parses");
    let mut state = State::new("books.parquet", project("p1"), &plan).unwrap();
    assert!(state
        .resume("books.parquet", &project("p1"), &mut plan)
        .is_ok());
    let message = refused(state.resume("books.parquet", &project("p2"), &mut plan));
    assert!(message.contains("--project-id p1"), "got: {message}");
}

#[test]
fn checkpoint_past_limit_is_rejected() {
    let mut plan: Spec = toml::from_str(&spec("limit = 5", "", "")).unwrap();
    let mut state = State::new("books.parquet", project("p1"), &plan).unwrap();
    state.cursors.insert(
        "a".to_string(),
        Mark::After(Checkpoint {
            cursor: Cursor::Key("6".to_string()),
            consumed: 6,
        }),
    );
    let error = refused(state.resume("books.parquet", &project("p1"), &mut plan));
    assert!(
        error.contains("checkpoint consumed 6 rows, exceeding limit 5"),
        "{error}"
    );
}
