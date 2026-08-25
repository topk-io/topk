mod common;

use assert_cmd::Command;
use bytesize::ByteSize;
use common::CliTestContext;
use test_context::test_context;
use topk::commands::list::ListEntry;

fn cmd() -> Command {
    Command::cargo_bin("topk").unwrap()
}

const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

#[test_context(CliTestContext)]
#[tokio::test]
async fn list_returns_uploaded_documents(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("list");
    ctx.create_dataset(&dataset);

    for pattern in ["pdfko.pdf", "markdown.md"] {
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o", "json", "upload", pattern, "-d", &dataset, "-y", "--wait",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // List and parse NDJSON
    let out = cmd()
        .args(["-o", "json", "list", "--dataset", &dataset])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let entries: Vec<ListEntry> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.size > ByteSize::b(0)));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn list_empty_dataset(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("list-empty");
    ctx.create_dataset(&dataset);

    let out = cmd()
        .args(["-o", "json", "list", "--dataset", &dataset])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let entries: Vec<ListEntry> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(entries.is_empty());
}
