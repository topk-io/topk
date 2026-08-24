mod common;

use assert_cmd::Command;
use common::{CliTestContext, OutputJsonExt};
use test_context::test_context;
use topk::commands::dataset::{
    CreateDatasetResult, Dataset, DeleteDatasetResult, GetDatasetResult, UpdateDatasetResult,
};

fn cmd() -> Command {
    Command::cargo_bin("topk").unwrap()
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn list(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    cmd()
        .args(["dataset", "create", "--region", &ctx.region, &name])
        .output()
        .unwrap();

    let out = cmd()
        .args(["-o", "json", "dataset", "list"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let datasets: Vec<Dataset> = out.json_lines().unwrap();
    let names: Vec<&str> = datasets.iter().map(|d| d.name.as_str()).collect();
    assert!(
        names.contains(&name.as_str()),
        "created dataset not in list: {:?}",
        names
    );
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn create(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    let out = cmd()
        .args([
            "-o",
            "json",
            "dataset",
            "create",
            "--region",
            &ctx.region,
            &name,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: CreateDatasetResult = out.json().unwrap();
    assert_eq!(result.dataset.name, name);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn create_with_description(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    let out = cmd()
        .args([
            "-o",
            "json",
            "dataset",
            "create",
            "--region",
            &ctx.region,
            "--description",
            "my dataset",
            &name,
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: CreateDatasetResult = out.json().unwrap();
    assert_eq!(result.dataset.name, name);
    assert_eq!(result.dataset.description.as_deref(), Some("my dataset"));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn get(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    cmd()
        .args(["dataset", "create", "--region", &ctx.region, &name])
        .output()
        .unwrap();

    let out = cmd()
        .args(["-o", "json", "dataset", "get", &name])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: GetDatasetResult = out.json().unwrap();
    assert_eq!(result.dataset.name, name);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn update(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    cmd()
        .args(["dataset", "create", "--region", &ctx.region, &name])
        .output()
        .unwrap();

    let out = cmd()
        .args([
            "-o",
            "json",
            "dataset",
            "update",
            &name,
            "--description",
            "Hello world",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: UpdateDatasetResult = out.json().unwrap();
    assert_eq!(result.dataset.name, name);
    assert_eq!(result.dataset.description.as_deref(), Some("Hello world"));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn update_without_fields(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    cmd()
        .args(["dataset", "create", "--region", &ctx.region, &name])
        .output()
        .unwrap();

    let out = cmd().args(["dataset", "update", &name]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("at least one field must be specified"));
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn delete(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    cmd()
        .args(["dataset", "create", "--region", &ctx.region, &name])
        .output()
        .unwrap();

    let out = cmd()
        .args(["-o", "json", "dataset", "delete", &name, "-y"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: DeleteDatasetResult = out.json().unwrap();
    assert!(result.deleted);
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn delete_aborted(ctx: &mut CliTestContext) {
    let name = ctx.wrap("test");
    cmd()
        .args(["dataset", "create", "--region", &ctx.region, &name])
        .output()
        .unwrap();

    let out = cmd()
        .args(["-o", "json", "dataset", "delete", &name])
        .write_stdin("wrong-name\n")
        .output()
        .unwrap();
    assert!(out.status.success());
    let result: DeleteDatasetResult = out.json().unwrap();
    assert!(!result.deleted);
}
