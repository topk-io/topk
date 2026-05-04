use colored::Colorize;
use futures::TryStreamExt;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use topk_rs::{
    proto::v1::ctx::{content, Content},
    Client, Error,
};

use crate::util::{mime::MimeType, read_query_from_stdin, value::value_to_json};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_type: String,
    pub dataset: String,
    pub content: Option<Content>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl From<topk_rs::proto::v1::ctx::SearchResult> for SearchResult {
    fn from(result: topk_rs::proto::v1::ctx::SearchResult) -> Self {
        Self {
            doc_id: result.doc_id,
            doc_type: result.doc_type,
            dataset: result.dataset,
            content: result.content,
            metadata: result
                .metadata
                .into_iter()
                .map(|(k, v)| (k, value_to_json(v)))
                .collect(),
        }
    }
}

impl SearchResults {
    pub fn render(&self, paths: &HashMap<String, PathBuf>) -> String {
        self.results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let ref_id = (i + 1).to_string();
                render_search_result(&ref_id, result, paths.get(&ref_id).map(PathBuf::as_path))
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(&HashMap::new()))
    }
}

#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    /// Search query (reads from stdin if omitted)
    pub query: Option<String>,
    /// Dataset to search (repeatable)
    #[arg(short = 'd', long = "dataset")]
    pub datasets: Vec<String>,
    /// Number of results to return
    #[arg(short = 'k', long, default_value = "10")]
    pub top_k: u32,
    /// Metadata fields to include in results (repeatable)
    #[arg(short = 'f', long = "field")]
    pub fields: Option<Vec<String>>,
    /// Save search results content (images, text chunks) to a directory
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<PathBuf>,
}

/// `topk search`
pub async fn run(client: &Client, args: &SearchArgs) -> Result<SearchResults, Error> {
    let query = match args.query.clone() {
        Some(query) => query,
        None => read_query_from_stdin()?,
    };

    Ok(SearchResults {
        results: client
            .search(
                query,
                args.datasets.clone(),
                args.top_k,
                None,
                args.fields.clone().unwrap_or_default(),
            )
            .await?
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .map(|r| r.into())
            .collect(),
    })
}

/// Write a search result content to a file
pub fn write_search_result(
    dir: &Path,
    ref_id: &str,
    result: &SearchResult,
) -> Result<PathBuf, Error> {
    let data = result
        .content
        .as_ref()
        .ok_or(Error::InvalidProto)?
        .data
        .as_ref()
        .ok_or(Error::InvalidProto)?;

    let ext = match data {
        content::Data::Chunk(_) => "txt".to_string(),
        content::Data::Image(img) => MimeType::from(img.mime_type.as_str()).to_ext().to_string(),
        content::Data::Page(page) => MimeType::from(
            page.image
                .as_ref()
                .ok_or(Error::InvalidProto)?
                .mime_type
                .as_str(),
        )
        .to_ext()
        .to_string(),
    };

    let path = dir.join(format!("{ref_id}.{ext}"));

    let bytes = match data {
        content::Data::Chunk(chunk) => chunk.text.as_bytes(),
        content::Data::Image(img) => img.data.as_ref(),
        content::Data::Page(page) => page
            .image
            .as_ref()
            .ok_or(Error::InvalidProto)?
            .data
            .as_ref(),
    };

    std::fs::create_dir_all(dir)?;

    std::fs::write(&path, bytes)?;

    Ok(path.canonicalize().unwrap_or(path))
}

pub fn save_search_results<'a, I, S>(
    output_dir: &Path,
    results: I,
) -> Result<HashMap<String, PathBuf>, Error>
where
    I: IntoIterator<Item = (S, &'a SearchResult)>,
    S: Into<String>,
{
    results
        .into_iter()
        .map(|(ref_id, result)| {
            let ref_id = ref_id.into();
            Ok::<_, Error>((
                ref_id.clone(),
                write_search_result(output_dir, &ref_id, result)?,
            ))
        })
        .collect()
}

pub fn render_search_result(ref_id: &str, result: &SearchResult, path: Option<&Path>) -> String {
    let text = match result.content.as_ref().and_then(|c| c.data.as_ref()) {
        Some(content::Data::Chunk(chunk)) => Some(chunk.text.to_string()),
        _ => None,
    };

    let placeholder = if path.is_none()
        && text.is_none()
        && !matches!(
            result.content.as_ref().and_then(|c| c.data.as_ref()),
            Some(content::Data::Chunk(_)) | None
        ) {
        format_content_text(result.content.as_ref())
    } else {
        None
    };

    let mut header = format!(
        "{} {}{} {}{} {}{}",
        format!("[{ref_id}]").blue(),
        "dataset=".dimmed(),
        result.dataset,
        "id=".dimmed(),
        result.doc_id,
        "type=".dimmed(),
        result.doc_type,
    );

    if let Some(path) = path {
        header.push_str(&format!(" {}{}", "file=".dimmed(), display_path(path)));
    }

    let mut lines = vec![header];

    let detail = match (text, placeholder) {
        (Some(t), _) => Some(t),
        (None, Some(p)) => Some(p),
        (None, None) => None,
    };

    if let Some(detail) = detail {
        lines.push(format!("{}", detail.dimmed()));
    }

    lines.join("\n")
}

fn display_path(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(relative) = path.strip_prefix(&cwd) {
            return relative.display().to_string();
        }
    }

    if let Some(file_name) = path.file_name() {
        if let Some(parent) = path.parent() {
            if parent == Path::new("") {
                return file_name.to_string_lossy().into_owned();
            }
        }
    };
    path.display().to_string()
}

pub fn format_content_text(content: Option<&Content>) -> Option<String> {
    match content.and_then(|c| c.data.as_ref()) {
        Some(content::Data::Chunk(chunk)) => {
            if chunk.doc_pages.is_empty() {
                Some(chunk.text.clone())
            } else {
                let pages: Vec<String> = chunk.doc_pages.iter().map(|p| p.to_string()).collect();
                Some(format!("{} [p.{}]", chunk.text, pages.join(",")))
            }
        }
        Some(content::Data::Page(page)) => Some(format!("<page {}>", page.page_number)),
        Some(content::Data::Image(img)) => Some(format!(
            "<image {} {}>",
            img.mime_type,
            bytesize::ByteSize(img.data.len() as u64)
        )),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::SearchResult;
    use assert_cmd::Command;
    use serde_json::json;
    use tempfile::tempdir;
    use test_context::test_context;
    use topk_rs::proto::v1::{ctx::file::InputFile, data::Value};

    use crate::commands::test_context::{CliTestContext, OutputJsonExt};

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn search_returns_results(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
        ctx.create_dataset(&dataset);

        let out = cmd()
            .args(["-o", "json", "search", "summarize", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let _: Vec<SearchResult> = out.json().unwrap();
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn search_json_output_saves_results_to_output_dir(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("json-output-dir");
        ctx.create_dataset(&dataset);

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upload",
                file,
                "--dataset",
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

        let dir = tempdir().unwrap();
        let out = cmd()
            .args([
                "-o",
                "json",
                "search",
                "Item one",
                "--dataset",
                &dataset,
                "--output-dir",
                dir.path().to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let result: Vec<SearchResult> = out.json().unwrap();
        assert!(!result.is_empty(), "expected search results");

        let saved_files = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();

        assert_eq!(saved_files.len(), result.len());
        for (index, _) in result.iter().enumerate() {
            let ref_id = (index + 1).to_string();
            assert!(
                saved_files
                    .iter()
                    .any(|path| path.file_stem() == Some(ref_id.as_ref())),
                "missing saved file for ref {ref_id}"
            );
        }
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn search_returns_metadata_fields(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("meta-fields");
        ctx.create_dataset(&dataset);

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let input = InputFile::from_path(file).unwrap();
        let upload = ctx
            .client
            .dataset(&dataset)
            .upsert_file(
                "meta-fields-doc",
                input,
                [
                    ("title", Value::string("My Test Document")),
                    ("author", Value::string("Test Author")),
                ],
            )
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
                "search",
                "test",
                "--dataset",
                &dataset,
                "--field",
                "title",
                "--field",
                "author",
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "search failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let result: Vec<SearchResult> = out.json().unwrap();
        let doc = result
            .iter()
            .find(|r| r.doc_id == "meta-fields-doc")
            .expect("document not found in search results");

        assert_eq!(doc.metadata.get("title"), Some(&json!("My Test Document")));
        assert_eq!(doc.metadata.get("author"), Some(&json!("Test Author")));
    }

    #[test]
    fn search_result_json_unwraps_metadata_values() {
        let result = topk_rs::proto::v1::ctx::SearchResult {
            doc_id: "doc1".to_string(),
            doc_type: "text/markdown".to_string(),
            dataset: "sec-10k".to_string(),
            content: None,
            metadata: [
                ("ticker".to_string(), Value::string("AAPL")),
                ("cik".to_string(), Value::i64(320193)),
            ]
            .into_iter()
            .collect(),
        };

        let json_result = SearchResult::from(result);

        assert_eq!(
            serde_json::to_value(json_result).unwrap(),
            json!({
                "doc_id": "doc1",
                "doc_type": "text/markdown",
                "dataset": "sec-10k",
                "content": null,
                "metadata": {
                    "ticker": "AAPL",
                    "cik": 320193
                }
            })
        );
    }
}
