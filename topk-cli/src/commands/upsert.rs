use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Map;
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
    pub metadata: Map<String, serde_json::Value>,
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

fn json_to_topk_value(v: &serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::null(),
        serde_json::Value::Bool(b) => Value::bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::i64(i)
            } else {
                Value::f64(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Value::string(s.clone()),
        _ => Value::null(),
    }
}

/// `topk upsert`
pub async fn run(
    client: &Client,
    dataset: &str,
    doc_id: DocId,
    file: PathBuf,
    metadata: Option<String>,
    dry_run: bool,
) -> Result<UpsertResult, Error> {
    let metadata: Map<String, serde_json::Value> = match metadata {
        Some(s) => serde_json::from_str(&s)
            .map_err(|e| Error::InvalidArgument(format!("invalid metadata JSON: {}", e)))?,
        None => Map::new(),
    };

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
                .map(|(k, v)| (k.clone(), json_to_topk_value(v))),
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
                "--id",
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
                "--id",
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
                "--id",
                "meta-doc",
                "--meta",
                r#"{"title": "Test Document", "author": "Test Author"}"#,
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
            result.metadata.get("title").and_then(|v| v.as_str()),
            Some("Test Document")
        );
        assert_eq!(
            result.metadata.get("author").and_then(|v| v.as_str()),
            Some("Test Author")
        );
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn upsert_metadata_types(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/pdfko.pdf");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upsert",
                "--dataset",
                &dataset,
                "--id",
                "types-doc",
                "--meta",
                r#"{"title": "My Doc", "pages": 42, "score": 3.14, "published": true, "tags": ["rust", "cli"], "counts": [1, 2, 3]}"#,
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

        assert_eq!(result.metadata.get("title").and_then(|v| v.as_str()), Some("My Doc"));
        assert_eq!(result.metadata.get("pages").and_then(|v| v.as_i64()), Some(42));
        assert!(result.metadata.get("score").and_then(|v| v.as_f64()).is_some());
        assert_eq!(result.metadata.get("published").and_then(|v| v.as_bool()), Some(true));

        let tags = result.metadata.get("tags").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tags[0].as_str(), Some("rust"));
        assert_eq!(tags[1].as_str(), Some("cli"));

        let counts = result.metadata.get("counts").and_then(|v| v.as_array()).unwrap();
        assert_eq!(counts[0].as_i64(), Some(1));
        assert_eq!(counts[2].as_i64(), Some(3));
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
                "--id",
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
