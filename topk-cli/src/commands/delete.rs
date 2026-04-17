use serde::{Deserialize, Serialize};
use topk_rs::{Client, Error};

use crate::output::{Output, RenderForHuman};

#[derive(Serialize, Deserialize)]
pub struct DeleteResult {
    deleted: bool,
    handle: Option<String>,
}

impl RenderForHuman for DeleteResult {
    fn render(&self) -> impl Into<String> {
        if self.deleted {
            "Document deleted.".to_string()
        } else {
            "Deletion skipped.".to_string()
        }
    }
}

/// `topk delete`
pub async fn run(
    client: &Client,
    dataset: &str,
    doc_id: impl Into<String>,
    yes: bool,
    output: &Output,
) -> Result<DeleteResult, Error> {
    let doc_id = doc_id.into();

    if !yes && !output.confirm(&format!("Delete document '{}'? [y/N] ", doc_id))? {
        return Ok(DeleteResult {
            deleted: false,
            handle: None,
        });
    }

    let result = client.dataset(dataset).delete(doc_id).await?;

    Ok(DeleteResult {
        deleted: true,
        handle: Some(result.into_inner().handle),
    })
}

#[cfg(test)]
mod tests {
    use super::DeleteResult;
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use test_context::test_context;
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
        let result: DeleteResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(result.deleted);
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
        let result: DeleteResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(!result.deleted);
    }
}
