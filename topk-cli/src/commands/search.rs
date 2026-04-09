use anyhow::Result;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use topk_rs::{
    proto::v1::ctx::{content, Content},
    Client,
};

use crate::output::RenderForHuman;

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    results: Vec<topk_rs::proto::v1::ctx::SearchResult>,
}

impl RenderForHuman for SearchResult {
    fn render(&self) -> String {
        if self.results.is_empty() {
            return "No results.".to_string();
        }
        self.results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "{}. {}\nDataset: {}\nDocument ID: {}\nDocument Type: {}",
                    i + 1,
                    format_content_text(r.content.as_ref()),
                    r.dataset,
                    r.doc_id,
                    r.doc_type,
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// `topk search`
pub async fn run(
    client: &Client,
    query: String,
    sources: Vec<String>,
    top_k: u32,
    fields: Option<Vec<String>>,
) -> Result<SearchResult> {
    let stream = client
        .search(query, sources, top_k, None, fields.unwrap_or_default())
        .await?
        .into_inner();

    let results = stream.try_collect().await?;
    Ok(SearchResult { results })
}

#[cfg(test)]
mod tests {
    use super::SearchResult;
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn search_returns_results(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        cmd()
            .args(["dataset", "create", &dataset])
            .output()
            .unwrap();

        let out = cmd()
            .args(["-o", "json", "search", "summarize"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _: SearchResult = serde_json::from_slice(&out.stdout).unwrap();
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn search_returns_metadata_fields(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("meta-fields");

        cmd()
            .args(["dataset", "create", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let out = cmd()
            .args([
                "-o", "json",
                "upsert",
                "--dataset",
                &dataset,
                "--document-id",
                "meta-fields-doc",
                "--meta",
                "title=My Test Document",
                "--meta",
                "author=Test Author",
                "--wait",
                file,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "upsert failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let out = cmd()
            .args([
                "-o", "json",
                "search",
                "test",
                "--dataset",
                &dataset,
                "--fields",
                "title,author",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "search failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let result: SearchResult = serde_json::from_slice(&out.stdout).unwrap();
        let doc = result
            .results
            .iter()
            .find(|r| r.doc_id == "meta-fields-doc")
            .expect("document not found in search results");

        assert_eq!(
            doc.metadata.get("title").and_then(|v| v.as_string()),
            Some("My Test Document"),
        );
        assert_eq!(
            doc.metadata.get("author").and_then(|v| v.as_string()),
            Some("Test Author"),
        );
    }
}

pub fn format_content_text(content: Option<&Content>) -> String {
    match content.and_then(|c| c.data.as_ref()) {
        Some(content::Data::Chunk(chunk)) => {
            if chunk.doc_pages.is_empty() {
                chunk.text.clone()
            } else {
                let pages: Vec<String> = chunk.doc_pages.iter().map(|p| p.to_string()).collect();
                format!("{} [p.{}]", chunk.text, pages.join(","))
            }
        }
        Some(content::Data::Page(page)) => format!("<page {}>", page.page_number),
        Some(content::Data::Image(img)) => format!(
            "<image {} {}>",
            img.mime_type,
            bytesize::ByteSize(img.data.len() as u64)
        ),
        None => String::new(),
    }
}
