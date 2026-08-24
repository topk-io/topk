mod common;

use assert_cmd::Command;
use common::{CliTestContext, OutputJsonExt};
use tempfile::tempdir;
use test_context::test_context;
use topk::commands::ask::AskResult;

fn cmd() -> Command {
    Command::cargo_bin("topk").unwrap()
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn ask_returns_result(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
    let out = cmd()
        .args([
            "-o",
            "json",
            "upload",
            file,
            "--dataset",
            &dataset,
            "-y",
            "--wait",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = cmd()
        .args(["-o", "json", "ask", "summarize", "--dataset", &dataset])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let _: AskResult = out.json().unwrap();
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn ask_json_output_saves_refs_to_output_dir(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("json-output-dir");
    ctx.create_dataset(&dataset);

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
    let out = cmd()
        .args([
            "-o",
            "json",
            "upload",
            file,
            "--dataset",
            &dataset,
            "-y",
            "--wait",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = tempdir().unwrap();
    let out = cmd()
        .args([
            "-o",
            "json",
            "ask",
            "What items are listed in section one?",
            "--dataset",
            &dataset,
            "--output-dir",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let result: AskResult = out.json().unwrap();
    assert!(!result.refs.is_empty(), "expected ask result references");

    let saved_files = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();

    assert_eq!(saved_files.len(), result.refs.len());
    for ref_id in result.refs.keys() {
        assert!(
            saved_files
                .iter()
                .any(|path| path.file_stem() == Some(ref_id.as_ref())),
            "missing saved file for ref {ref_id}"
        );
    }
}
