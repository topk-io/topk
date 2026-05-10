use colored::Colorize;
use futures::TryStreamExt;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use topk_rs::{Client, Error};

use crate::util::{mime::MimeType, read_query_from_stdin, value::value_to_json, Base64};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_type: String,
    pub dataset: String,
    pub content_id: String,
    pub doc_name: String,
    pub content: Option<Content>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Content {
    Chunk {
        text: String,
        doc_pages: Vec<u32>,
    },
    Image(Image),
    Page {
        page_number: u32,
        image: Option<Image>,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Image {
    pub mime_type: String,
    pub data: Base64,
}

impl From<topk_rs::proto::v1::ctx::SearchResult> for SearchResult {
    fn from(result: topk_rs::proto::v1::ctx::SearchResult) -> Self {
        let content = match result.content {
            None => None,
            Some(proto) => match proto.data {
                None => None,
                Some(data) => Some(Content::from(data)),
            },
        };

        Self {
            doc_id: result.doc_id,
            doc_type: result.doc_type,
            dataset: result.dataset,
            content_id: result.content_id,
            doc_name: result.doc_name,
            content,
            metadata: result
                .metadata
                .into_iter()
                .map(|(k, v)| (k, value_to_json(v)))
                .collect(),
        }
    }
}

impl From<topk_rs::proto::v1::ctx::content::Data> for Content {
    fn from(data: topk_rs::proto::v1::ctx::content::Data) -> Self {
        match data {
            topk_rs::proto::v1::ctx::content::Data::Chunk(chunk) => Self::Chunk {
                text: chunk.text,
                doc_pages: chunk.doc_pages,
            },
            topk_rs::proto::v1::ctx::content::Data::Image(image) => Self::Image(Image {
                mime_type: image.mime_type,
                data: image.data.into(),
            }),
            topk_rs::proto::v1::ctx::content::Data::Page(page) => Self::Page {
                page_number: page.page_number,
                image: page.image.map(|image| Image {
                    mime_type: image.mime_type,
                    data: image.data.into(),
                }),
            },
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
            .collect::<Vec<_>>(),
    })
}

/// Save search results to a directory
pub fn save_search_results(
    output_dir: &Path,
    refs: &HashMap<String, SearchResult>,
) -> Result<HashMap<String, PathBuf>, Error> {
    std::fs::create_dir_all(output_dir)?;

    let mut paths = HashMap::new();
    for (ref_id, result) in refs {
        let content = result.content.as_ref().ok_or(Error::InvalidProto)?;

        let (ext, bytes): (String, &[u8]) = match content {
            Content::Chunk { text, .. } => ("txt".to_string(), text.as_bytes()),
            Content::Image(img) => (
                MimeType::from(img.mime_type.as_str()).to_ext().to_string(),
                img.data.0.as_ref(),
            ),
            Content::Page { image, .. } => {
                let img = image.as_ref().ok_or(Error::InvalidProto)?;
                (
                    MimeType::from(img.mime_type.as_str()).to_ext().to_string(),
                    img.data.0.as_ref(),
                )
            }
        };

        let path = output_dir.join(format!("{ref_id}.{ext}"));

        std::fs::write(&path, bytes)?;

        paths.insert(ref_id.clone(), path.canonicalize().unwrap_or(path));
    }

    Ok(paths)
}

pub fn render_search_result(ref_id: &str, result: &SearchResult, path: Option<&Path>) -> String {
    let text = match &result.content {
        Some(Content::Chunk { text, .. }) => Some(text.to_string()),
        _ => None,
    };

    let placeholder = if path.is_none()
        && text.is_none()
        && !matches!(&result.content, Some(Content::Chunk { .. }))
    {
        result.content.as_ref().and_then(format_content_text)
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

pub fn format_content_text(content: &Content) -> Option<String> {
    match content {
        Content::Chunk { text, doc_pages } => {
            if doc_pages.is_empty() {
                Some(text.clone())
            } else {
                let pages: Vec<String> = doc_pages.iter().map(|p| p.to_string()).collect();
                Some(format!("{} [p.{}]", text, pages.join(",")))
            }
        }
        Content::Page { page_number, .. } => Some(format!("<page {}>", page_number)),
        Content::Image(img) => Some(format!(
            "<image {} {}>",
            img.mime_type,
            bytesize::ByteSize(img.data.0.len() as u64)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Base64, Content, Image, SearchResult};
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
            doc_name: "doc1.md".to_string(),
            dataset: "sec-10k".to_string(),
            content_id: "chunk-1".to_string(),
            content: Some(topk_rs::proto::v1::ctx::Content {
                data: Some(topk_rs::proto::v1::ctx::content::Data::Chunk(topk_rs::proto::v1::ctx::Chunk {
                    text: "hello".to_string(),
                    doc_pages: vec![],
                })),
            }),
            metadata: [
                ("ticker".to_string(), Value::string("AAPL")),
                ("cik".to_string(), Value::i64(320193)),
            ]
            .into_iter()
            .collect(),
        };

        let json_result = SearchResult::try_from(result).unwrap();

        assert_eq!(
            serde_json::to_value(json_result).unwrap(),
            json!({
                "doc_id": "doc1",
                "doc_type": "text/markdown",
                "doc_name": "doc1.md",
                "dataset": "sec-10k",
                "content_id": "chunk-1",
                "content": {
                    "text": "hello",
                    "doc_pages": []
                },
                "metadata": {
                    "ticker": "AAPL",
                    "cik": 320193
                }
            })
        );
    }

    #[test]
    fn search_result_json_flattens_chunk_content() {
        let result = SearchResult {
            doc_id: "doc1".to_string(),
            doc_type: "application/pdf".to_string(),
            dataset: "sec-10k".to_string(),
            content_id: "chunk-1".to_string(),
            doc_name: "doc1.pdf".to_string(),
            content: Some(Content::Chunk {
                text: "hello".to_string(),
                doc_pages: vec![170],
            }),
            metadata: serde_json::Map::new(),
        };

        assert_eq!(
            serde_json::to_value(result).unwrap(),
            json!({
                "doc_id": "doc1",
                "doc_type": "application/pdf",
                "dataset": "sec-10k",
                "content_id": "chunk-1",
                "doc_name": "doc1.pdf",
                "content": {
                    "text": "hello",
                    "doc_pages": [170]
                }
            })
        );
    }

    #[test]
    fn search_result_json_encodes_image_bytes_as_base64() {
        let result = SearchResult {
            doc_id: "doc1".to_string(),
            doc_type: "image/png".to_string(),
            dataset: "images".to_string(),
            content_id: "img-1".to_string(),
            doc_name: "doc1.png".to_string(),
            content: Some(Content::Image(Image {
                mime_type: "image/png".to_string(),
                data: bytes::Bytes::from(vec![1, 2, 3]).into(),
            })),
            metadata: serde_json::Map::new(),
        };

        assert_eq!(
            serde_json::to_value(result).unwrap(),
            json!({
                "doc_id": "doc1",
                "doc_type": "image/png",
                "dataset": "images",
                "content_id": "img-1",
                "doc_name": "doc1.png",
                "content": {
                    "mime_type": "image/png",
                    "data": "AQID"
                }
            })
        );
    }
}
