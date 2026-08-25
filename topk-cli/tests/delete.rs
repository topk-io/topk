mod common;

use assert_cmd::Command;
use common::{CliTestContext, OutputJsonExt};
use test_context::test_context;
use topk::commands::delete::DeleteResult;
use topk_rs::proto::v1::{ctx::file::InputFile, data::Value};

fn cmd() -> Command {
    Command::cargo_bin("topk").unwrap()
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn delete_document(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
    let input = InputFile::from_path(file).unwrap();
    let upload = ctx
        .client
        .dataset(&dataset)
        .upsert_file("doc-to-delete", input, Vec::<(String, Value)>::new())
        .await
        .unwrap();
    ctx.client
        .dataset(&dataset)
        .wait_for_handle(&upload, None)
        .await
        .unwrap();

    let out = cmd()
        .args([
            "-o",
            "json",
            "delete",
            "-d",
            &dataset,
            "--id",
            "doc-to-delete",
            "-y",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: DeleteResult = out.json().unwrap();
    assert!(result.handle.is_some());
}

#[test_context(CliTestContext)]
#[tokio::test]
async fn delete_aborted(ctx: &mut CliTestContext) {
    let dataset = ctx.wrap("test");
    ctx.create_dataset(&dataset);

    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
    let input = InputFile::from_path(file).unwrap();
    let upload = ctx
        .client
        .dataset(&dataset)
        .upsert_file("doc-to-keep", input, Vec::<(String, Value)>::new())
        .await
        .unwrap();
    ctx.client
        .dataset(&dataset)
        .wait_for_handle(&upload, None)
        .await
        .unwrap();

    // --json mode is non-interactive so confirm returns false → skipped
    let out = cmd()
        .args([
            "-o",
            "json",
            "delete",
            "-d",
            &dataset,
            "--id",
            "doc-to-keep",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: DeleteResult = out.json().unwrap();
    assert!(result.handle.is_none());
}
