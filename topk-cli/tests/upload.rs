mod common;

use assert_cmd::Command;
use common::{CliTestContext, OutputJsonExt};
use std::fs;
use tempfile::tempdir;
use test_context::test_context;
use topk::commands::upload::UploadResult;
use uuid::Uuid;

fn cmd() -> Command {
    Command::cargo_bin("topk").unwrap()
}

const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

#[test_context(CliTestContext)]
#[tokio::test]
async fn wait_timeout_aborts(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("timeout");
    ctx.create_dataset(&dataset);
    let out = cmd()
        .current_dir(TESTS_DIR)
        .args([
            "-o",
            "json",
            "upload",
            "pdfko.pdf",
            "-y",
            "--dataset",
            &dataset,
            "--wait",
            "0s",
        ])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "expected failure due to timeout, got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("retry timeout"), "{stderr}");
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_single_file(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);
    let out = cmd()
        .current_dir(TESTS_DIR)
        .args([
            "-o",
            "json",
            "upload",
            "pdfko.pdf",
            "-y",
            "--dataset",
            &dataset,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: UploadResult = out.json().unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.uploaded, 1);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_dry_run(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);
    let out = cmd()
        .current_dir(TESTS_DIR)
        .args([
            "-o",
            "json",
            "upload",
            "pdfko.pdf",
            "markdown.md",
            "--dataset",
            &dataset,
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: UploadResult = out.json().unwrap();
    assert_eq!(result.total, 2);
    assert_eq!(result.uploaded, 0);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_with_yes_skips_confirmation(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("yes-flag");
    ctx.create_dataset(&dataset);
    let out = cmd()
        .current_dir(TESTS_DIR)
        .args(["-o", "json", "upload", "pdfko.pdf", "-d", &dataset, "-y"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: UploadResult = out.json().unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.uploaded, 1);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_recursive(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);

    let dir = tempdir().unwrap();
    let nested = dir.path().join("sub");
    fs::create_dir(&nested).unwrap();
    fs::write(dir.path().join("top.md"), "# top").unwrap();
    fs::write(nested.join("deep.md"), "# deep").unwrap();
    fs::write(nested.join("skip.txt"), "skip").unwrap();

    let out = cmd()
        .current_dir(dir.path())
        .args(["-o", "json", "upload", "*.md", "-d", &dataset, "--dry-run"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: UploadResult = out.json().unwrap();
    assert_eq!(result.total, 1);
    assert_eq!(result.uploaded, 0);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_recursive_with_globstar_pattern(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);

    let dir = tempdir().unwrap();
    let nested = dir.path().join("sub");
    fs::create_dir(&nested).unwrap();
    fs::write(dir.path().join("top.md"), "# top").unwrap();
    fs::write(nested.join("deep.md"), "# deep").unwrap();
    fs::write(nested.join("skip.txt"), "skip").unwrap();

    let out = cmd()
        .current_dir(dir.path())
        .args([
            "-o",
            "json",
            "upload",
            "**/*.md",
            "-d",
            &dataset,
            "--dry-run",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: UploadResult = out.json().unwrap();
    assert_eq!(result.total, 2);
    assert_eq!(result.uploaded, 0);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_wait(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.client
        .datasets()
        .create(&dataset, Some(ctx.region.clone()), None)
        .await
        .unwrap();

    let out = cmd()
        .current_dir(TESTS_DIR)
        .args([
            "-o",
            "json",
            "upload",
            "pdfko.pdf",
            "-d",
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
    let result: UploadResult = out.json().unwrap();
    assert_eq!(result.uploaded, 1);
    assert_eq!(result.processed, Some(true));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn upload_requires_existing_dataset(ctx: &mut CliTestContext) {
    let dataset = format!("{}-missing-{}", ctx.scope, Uuid::new_v4().simple());
    let out = cmd()
        .current_dir(TESTS_DIR)
        .args(["-o", "json", "upload", "pdfko.pdf", "--dataset", &dataset])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not found"), "{stderr}");
}
