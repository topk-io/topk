use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;
use topk_rs::{
    Client, Error,
    proto::v1::{ctx::{doc::DocId, file::InputFile}, data::Value},
};

use crate::output::RenderForHuman;

#[derive(Serialize, serde::Deserialize)]
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

    let result = client.dataset(dataset).upsert_file(doc_id, input, meta).await?;

    Ok(UpsertResult { handle: result.into_inner().handle, processed: false })
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use uuid::Uuid;
    use super::UpsertResult;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    fn unique_name() -> String {
        format!("topk-cli-{}", Uuid::new_v4().simple())
    }

    fn create_dataset(name: &str) {
        cmd().args(["dataset", "create", "--dataset", name]).output().unwrap();
    }

    fn delete_dataset(name: &str) {
        cmd().args(["dataset", "delete", "--dataset", name, "-y"]).output().unwrap();
    }

    #[test]
    fn upsert_pdf() {
        let dataset = unique_name();
        create_dataset(&dataset);

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args(["--json", "upsert", "--dataset", &dataset, "--document-id", "test-doc", file])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let result: UpsertResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(!result.handle.is_empty());

        delete_dataset(&dataset);
    }

    #[test]
    fn upsert_markdown() {
        let dataset = unique_name();
        create_dataset(&dataset);

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let out = cmd()
            .args(["--json", "upsert", "--dataset", &dataset, "--document-id", "test-doc", file])
            .output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let result: UpsertResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(!result.handle.is_empty());

        delete_dataset(&dataset);
    }
}
