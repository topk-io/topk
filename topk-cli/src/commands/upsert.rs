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
) -> Result<UpsertResult, Error> {
    let input = InputFile::from_path(&file)?;
    let meta: HashMap<String, Value> = metadata
        .into_iter()
        .map(|(k, v)| (k, Value::string(v)))
        .collect();

    let result = client
        .dataset(dataset)
        .upsert_file(doc_id, input, meta)
        .await?;

    Ok(UpsertResult {
        handle: result.into_inner().handle,
        processed: false,
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
            .args(["dataset", "create", "--dataset", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args([
                "--json",
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
            .args(["dataset", "create", "--dataset", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let out = cmd()
            .args([
                "--json",
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
}
