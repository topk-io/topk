use std::fmt;

use serde::{Deserialize, Serialize};
use topk_rs::{Client, Error};

use crate::output::Output;

#[derive(Serialize, Deserialize)]
pub struct DeleteResult {
    deleted: bool,
    handle: Option<String>,
}

impl fmt::Display for DeleteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.deleted {
            f.write_str("Document deleted.")
        } else {
            f.write_str("Deletion skipped.")
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    /// Dataset name
    #[arg(short = 'd', long, value_name = "DATASET_NAME")]
    pub dataset: String,
    /// Document ID
    #[arg(long)]
    pub id: String,
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// `topk delete`
pub async fn run(
    client: &Client,
    args: &DeleteArgs,
    output: &Output,
) -> Result<DeleteResult, Error> {
    if !output.confirm_or_yes(&format!("Delete document '{}'? ", args.id), args.yes)? {
        return Ok(DeleteResult {
            deleted: false,
            handle: None,
        });
    }

    let handle = client
        .dataset(&args.dataset)
        .delete(args.id.clone())
        .await?
        .into_inner()
        .handle;

    Ok(DeleteResult {
        deleted: true,
        handle: Some(handle),
    })
}

#[cfg(test)]
mod tests {
    use super::DeleteResult;
    use crate::commands::test_context::{CliTestContext, OutputJsonExt};
    use assert_cmd::Command;
    use test_context::test_context;
    use topk_rs::proto::v1::{ctx::file::InputFile, data::Value};

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
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
            .wait_for_handle(&upload.handle, None)
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
        assert!(result.deleted);
        assert!(result.handle.is_some());
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
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
            .wait_for_handle(&upload.handle, None)
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
        assert!(!result.deleted);
    }
}
