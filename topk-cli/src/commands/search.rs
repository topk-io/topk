use std::fmt;

use anyhow::Result;
use colored::Colorize;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use topk_rs::{
    proto::v1::ctx::{content, Content},
    Client, Error,
};

use crate::output::Output;
use crate::util::{mime::MimeType, resolve_query};

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub result: topk_rs::proto::v1::ctx::SearchResult,
    #[serde(skip, default)]
    pub path: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResults {
    results: Vec<SearchResult>,
}

impl fmt::Display for SearchResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.results.is_empty() {
            return f.write_str("No results.");
        }

        let entries: Vec<String> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, r)| render_search_result(&(i + 1).to_string(), r, None))
            .collect();

        f.write_str(&entries.join("\n\n"))
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
pub async fn run(
    client: &Client,
    args: &SearchArgs,
    output: &Output,
) -> Result<SearchResults, Error> {
    let query = resolve_query(args.query.clone())
        .map_err(|e| Error::Internal(e.to_string()))?
        .ok_or_else(|| {
            Error::Input(anyhow::anyhow!(
                "query is required; pass it as an argument or pipe it via stdin"
            ))
        })?;

    let raw: Vec<_> = client
        .search(
            query,
            args.datasets.clone(),
            args.top_k,
            None,
            args.fields.clone().unwrap_or_default(),
        )
        .await?
        .into_inner()
        .try_collect()
        .await?;

    let output_dir = match &args.output_dir {
        Some(dir) => Some(dir.clone()),
        None if !output.is_json() => {
            let ids_str = raw
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    !matches!(
                        r.content.as_ref().and_then(|c| c.data.as_ref()),
                        Some(content::Data::Chunk(_)) | None
                    )
                })
                .map(|(i, _)| format!("{}", format!("[{}]", i + 1).blue()))
                .collect::<Vec<_>>()
                .join(", ");
            if ids_str.is_empty() {
                None
            } else {
                output.prompt_dir(format!("{ids_str} contain non-text citations. Save to directory (or Enter to skip)")).map_err(Error::IoError)?
            }
        }
        None => None,
    };

    let results: Vec<SearchResult> = raw
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            let path = if let Some(ref dir) = output_dir {
                write_result_content(dir, &(i + 1).to_string(), &v).map_err(Error::IoError)?
            } else {
                None
            };
            Ok::<_, Error>(SearchResult { result: v, path })
        })
        .collect::<Result<_, _>>()?;

    if let Some(ref dir) = output_dir {
        let saved = results
            .iter()
            .filter(|r: &&SearchResult| r.path.is_some())
            .count();
        if saved > 0 {
            output.success(&format!("Saved {saved} file(s) to {}", dir.display()));
        }
    }

    Ok(SearchResults { results })
}

/// Write a single result's content to `dir/<name>.<ext>`. Returns the path on success,
/// or `None` if there is no saveable content.
pub fn write_result_content(
    dir: &Path,
    name: &str,
    result: &topk_rs::proto::v1::ctx::SearchResult,
) -> std::io::Result<Option<PathBuf>> {
    let data = match result.content.as_ref().and_then(|c| c.data.as_ref()) {
        Some(d) => d,
        None => return Ok(None),
    };
    std::fs::create_dir_all(dir)?;
    let path = match data {
        content::Data::Chunk(chunk) => {
            let path = dir.join(format!("{name}.txt"));
            std::fs::write(&path, chunk.text.as_bytes())?;
            path
        }
        content::Data::Image(img) => {
            let path = dir.join(format!(
                "{name}.{}",
                MimeType::from(img.mime_type.as_str()).to_ext()
            ));
            std::fs::write(&path, &img.data)?;
            path
        }
        content::Data::Page(page) => match &page.image {
            Some(img) => {
                let path = dir.join(format!(
                    "{name}.{}",
                    MimeType::from(img.mime_type.as_str()).to_ext()
                ));
                std::fs::write(&path, &img.data)?;
                path
            }
            None => return Ok(None),
        },
    };
    Ok(Some(path.canonicalize().unwrap_or(path)))
}

pub fn render_search_result(id: &str, r: &SearchResult, max_text_len: Option<usize>) -> String {
    let text = format_reference_detail(r.result.content.as_ref()).map(|t| match max_text_len {
        Some(max) if t.chars().count() > max => {
            format!("{}…", t.chars().take(max).collect::<String>())
        }
        _ => t,
    });
    let placeholder = if r.path.is_none()
        && text.is_none()
        && !matches!(
            r.result.content.as_ref().and_then(|c| c.data.as_ref()),
            Some(content::Data::Chunk(_)) | None
        ) {
        format_content_text(r.result.content.as_ref())
    } else {
        None
    };
    let detail = match (&r.path, text) {
        (Some(path), Some(t)) => Some(format!("{}", format!("{}\n{t}", path.display()).dimmed())),
        (Some(path), None) => Some(format!("{}", format!("{}", path.display()).dimmed())),
        (None, Some(t)) => Some(format!("{}", t.dimmed())),
        (None, None) => placeholder.map(|p| format!("{}", p.dimmed())),
    };
    [format!(
        "{} {}, {}, {}",
        format!("[{id}]").blue(),
        r.result.dataset,
        r.result.doc_id,
        r.result.doc_type
    )]
    .into_iter()
    .chain(detail)
    .collect::<Vec<_>>()
    .join("\n")
}

fn format_reference_detail(content: Option<&Content>) -> Option<String> {
    match content.and_then(|c| c.data.as_ref()) {
        Some(content::Data::Chunk(chunk)) => {
            let text = chunk.text.trim();
            if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            }
        }
        _ => None,
    }
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
    use super::SearchResults;
    use crate::commands::test_context::{CliTestContext, OutputJsonExt};
    use assert_cmd::Command;
    use test_context::test_context;
    use topk_rs::proto::v1::{ctx::file::InputFile, data::Value};

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
        let _: SearchResults = out.json().unwrap();
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
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
            .wait_for_handle(&upload.handle, None)
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

        let result: SearchResults = out.json().unwrap();
        let doc = result
            .results
            .iter()
            .find(|r| r.result.doc_id == "meta-fields-doc")
            .expect("document not found in search results");

        assert_eq!(
            doc.result.metadata.get("title").and_then(|v| v.as_string()),
            Some("My Test Document"),
        );
        assert_eq!(
            doc.result
                .metadata
                .get("author")
                .and_then(|v| v.as_string()),
            Some("Test Author"),
        );
    }
}
