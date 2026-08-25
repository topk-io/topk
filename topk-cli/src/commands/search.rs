use std::collections::{BTreeMap, HashMap};

use colored::Colorize;
use futures::TryStreamExt;
use std::fmt;
use std::path::{Path, PathBuf};
use topk_rs::json::Value;
use topk_rs::{Client, Error};

use crate::util::{mime::MimeType, read_query_from_stdin, Base64};

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
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
        let content = result
            .content
            .and_then(|proto| proto.data)
            .map(Content::from);

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
                .map(|(k, v)| (k, Value::from(v)))
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
            .collect(),
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
                img.data.as_ref(),
            ),
            Content::Page { image, .. } => {
                let img = image.as_ref().ok_or(Error::InvalidProto)?;
                (
                    MimeType::from(img.mime_type.as_str()).to_ext().to_string(),
                    img.data.as_ref(),
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
            bytesize::ByteSize(img.data.as_ref().len() as u64)
        )),
    }
}
