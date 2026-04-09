use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use topk_rs::{
    proto::v1::{
        ctx::{doc::DocId, file::InputFile},
        data::Value,
    },
    Client, Error,
};

use crate::output::RenderForHuman;

#[derive(Serialize, Deserialize)]
pub struct UpsertResult {
    pub handle: String,
    pub processed: bool,
    pub metadata: HashMap<String, String>,
}

impl RenderForHuman for UpsertResult {
    fn render(&self) -> String {
        if self.processed {
            "Uploaded and processed.".to_string()
        } else {
            "Uploaded.".to_string()
        }
    }
}

/// `topk upsert`
pub async fn run(
    client: &Client,
    dataset: &str,
    doc_id: DocId,
    file: PathBuf,
    metadata: Vec<(String, String)>,
    dry_run: bool,
) -> Result<UpsertResult, Error> {
    let metadata: HashMap<String, String> = metadata.into_iter().collect();

    if dry_run {
        return Ok(UpsertResult {
            handle: "dry-run".to_string(),
            processed: false,
            metadata,
        });
    }

    let result = client
        .dataset(dataset)
        .upsert_file(
            doc_id,
            InputFile::from_path(&file)?,
            metadata
                .iter()
                .map(|(k, v)| (k.clone(), Value::string(v.clone()))),
        )
        .await?;

    Ok(UpsertResult {
        handle: result.into_inner().handle,
        processed: false,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::UpsertResult;
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upsert_pdf(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upsert",
                "--dataset",
                &dataset,
                "--document-id",
                "test-doc",
                file,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UpsertResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(!result.handle.is_empty());
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upsert_markdown(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upsert",
                "--dataset",
                &dataset,
                "--document-id",
                "test-doc",
                file,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UpsertResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(!result.handle.is_empty());
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upsert_with_metadata(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upsert",
                "--dataset",
                &dataset,
                "--document-id",
                "meta-doc",
                "--meta",
                "title=Test Document",
                "--meta",
                "author=Test Author",
                "--dry-run",
                file,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UpsertResult = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            result.metadata.get("title").map(|s| s.as_str()),
            Some("Test Document")
        );
        assert_eq!(
            result.metadata.get("author").map(|s| s.as_str()),
            Some("Test Author")
        );
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
    async fn upsert_wait(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", "-d", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upsert",
                "-d",
                &dataset,
                "--document-id",
                "wait-doc",
                "--wait",
                file,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let result: UpsertResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(result.processed);
        assert!(!result.handle.is_empty());
    }
}
