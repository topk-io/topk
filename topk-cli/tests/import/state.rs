use std::collections::BTreeMap;

use crate::common::*;
use topk::import::{Cursor, Mark, Spec, State};
use topk::ProjectId;

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
    let mut state = State::new(
        "run1".to_string(),
        "books.parquet".to_string(),
        stored,
        None,
    );
    state
        .cursors
        .insert("a".to_string(), Mark::After(Cursor::Key("100".to_string())));
    state
        .cursors
        .insert("b".to_string(), Mark::After(Cursor::Key("200".to_string())));
    state.cursors.insert("c".to_string(), Mark::Done);

    let edited = spec("limit = 5", "", "");
    let mut plan: Spec = toml::from_str(&edited).expect("spec parses");
    let (done, after) = state
        .reconcile("books.parquet", None, &mut plan, edited)
        .expect("same source reconciles");

    assert_eq!(done, 1, "c was already imported");
    assert_eq!(
        after,
        BTreeMap::from([("b".to_string(), Cursor::Key("200".to_string()))]),
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
    let mut state = State::new(
        "run1".to_string(),
        "books.parquet".to_string(),
        stored.clone(),
        None,
    );
    let mut plan: Spec = toml::from_str(&stored).expect("spec parses");
    let message = refused(state.reconcile("other.parquet", None, &mut plan, stored));
    assert!(message.contains("books.parquet"), "got: {message}");
}

#[test]
fn cursors_round_trip_in_run_state() {
    let project_id = "p1".parse::<ProjectId>().unwrap();
    let mut state = State::new(
        "run1".to_string(),
        "books.parquet".to_string(),
        spec("", "", ""),
        Some(project_id.clone()),
    );
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
        state
            .cursors
            .insert((*name).to_string(), Mark::After(cursor.clone()));
    }

    let encoded = toml::to_string_pretty(&state).expect("state serializes");
    let decoded: State = toml::from_str(&encoded).expect("state deserializes");
    assert_eq!(decoded.project_id, Some(project_id), "{encoded}");
    for (name, expected) in cursors {
        let Some(Mark::After(actual)) = decoded.cursors.get(name) else {
            panic!("missing cursor {name:?} in {encoded}");
        };
        assert_eq!(actual, &expected, "{encoded}");
    }
}

/// A resumed run keeps the project it was started for.
#[test]
fn a_resume_cannot_switch_projects() {
    let started = "p1".parse::<ProjectId>().unwrap();
    let other = "p2".parse::<ProjectId>().unwrap();
    let stored = spec("", "", "");
    let reconcile = |project_id: Option<ProjectId>, requested: Option<&ProjectId>| {
        let mut state = State::new(
            "run1".to_string(),
            "books.parquet".to_string(),
            stored.clone(),
            project_id,
        );
        let mut plan: Spec = toml::from_str(&stored).expect("spec parses");
        state.reconcile("books.parquet", requested, &mut plan, stored.clone())
    };
    assert!(reconcile(Some(started.clone()), Some(&started)).is_ok());
    for requested in [Some(&other), None] {
        let message = refused(reconcile(Some(started.clone()), requested));
        assert!(message.contains("--project-id p1"), "got: {message}");
    }

    assert!(reconcile(None, None).is_ok());
    let message = refused(reconcile(None, Some(&started)));
    assert!(message.contains("--api-key"), "got: {message}");
}
