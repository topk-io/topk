use anyhow::Result;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use terminal_size::{terminal_size, Width as TermWidth};
use topk_rs::{
    proto::v1::ctx::{content, Content},
    Client, Error,
};

use crate::output::{Output, RenderForHuman, BLUE, BOLD, DIM, RESET};
use crate::util::MimeType;

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    results: Vec<topk_rs::proto::v1::ctx::SearchResult>,
    #[serde(skip, default)]
    saved: Vec<Option<PathBuf>>,
}

impl RenderForHuman for SearchResult {
    fn render(&self) -> impl Into<String> {
        if self.results.is_empty() {
            return "No results.".to_string();
        }

        let entries: Vec<String> = self
            .results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let saved = self.saved.get(i).and_then(|p| p.as_ref());
                render_search_result(&(i + 1).to_string(), r, saved, None)
            })
            .collect();

        entries.join("\n\n")
    }
}

/// `topk search`
pub async fn run(
    client: &Client,
    query: String,
    datasets: Vec<String>,
    top_k: u32,
    fields: Option<Vec<String>>,
    output_dir: Option<PathBuf>,
    output: &Output,
) -> Result<(), Error> {
    let stream = client
        .search(query, datasets, top_k, None, fields.unwrap_or_default())
        .await?
        .into_inner();

    let results: Vec<_> = stream.try_collect().await.map_err(Error::from)?;
    let mut result = SearchResult {
        results,
        saved: vec![],
    };

    if !output.is_human() {
        if let Some(ref dir) = output_dir {
            result.saved = save_results(dir, &result.results)?;
        }
        output
            .print_json(&result)
            .map_err(|e| Error::Internal(e.to_string()))?;
        return Ok(());
    }

    if let Some(ref dir) = output_dir {
        result.saved = save_results(dir, &result.results)?;
        output
            .print_human(&result)
            .map_err(|e| Error::Internal(e.to_string()))?;
        return Ok(());
    }

    let initial_rendered: String = result.render().into();
    if !initial_rendered.is_empty() {
        println!("{initial_rendered}");
    }

    let output_dir = prompt_for_output_dir(&result.results)?;
    if let Some(dir) = output_dir {
        result.saved = save_results(&dir, &result.results)?;
        let rerendered: String = result.render().into();
        if !rerendered.is_empty() {
            replace_rendered_stdout(rendered_display_line_count(&initial_rendered), &rerendered)?;
        }
    }

    Ok(())
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
            let mime = MimeType::from(img.mime_type.as_str());
            let path = dir.join(format!("{name}.{}", mime.to_ext()));
            std::fs::write(&path, &img.data)?;
            path
        }
        _ => return Ok(None),
    };
    Ok(Some(path.canonicalize().unwrap_or(path)))
}

pub fn render_search_result(
    id: &str,
    r: &topk_rs::proto::v1::ctx::SearchResult,
    saved: Option<&PathBuf>,
    max_text_len: Option<usize>,
) -> String {
    let text = format_reference_detail(r.content.as_ref()).map(|t| match max_text_len {
        Some(max) if t.chars().count() > max => {
            format!("{}…", t.chars().take(max).collect::<String>())
        }
        _ => t,
    });
    let placeholder = if saved.is_none() && text.is_none() && has_saveable_content(r) {
        Some(format_content_text(r.content.as_ref()))
    } else {
        None
    };
    let detail = match (saved, text) {
        (Some(path), Some(t)) => Some(format!("{DIM}{}\n{t}{RESET}", path.display())),
        (Some(path), None) => Some(format!("{DIM}{}{RESET}", path.display())),
        (None, Some(t)) => Some(format!("{DIM}{t}{RESET}")),
        (None, None) => placeholder.map(|p| format!("{DIM}{p}{RESET}")),
    };
    [format!(
        "{BLUE}[{id}]{RESET} {}, {}, {}",
        r.dataset, r.doc_id, r.doc_type
    )]
    .into_iter()
    .chain(detail)
    .collect::<Vec<_>>()
    .join("\n")
}

/// Returns true if the result has content that can be saved to disk but not displayed inline.
pub fn has_saveable_content(r: &topk_rs::proto::v1::ctx::SearchResult) -> bool {
    matches!(
        r.content.as_ref().and_then(|c| c.data.as_ref()),
        Some(content::Data::Image(_))
    )
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

fn save_results(
    dir: &Path,
    results: &[topk_rs::proto::v1::ctx::SearchResult],
) -> Result<Vec<Option<PathBuf>>, Error> {
    let paths: Vec<Option<PathBuf>> = results
        .iter()
        .enumerate()
        .map(|(i, r)| write_result_content(dir, &(i + 1).to_string(), r).map_err(Error::IoError))
        .collect::<Result<_, _>>()?;
    let count = paths.iter().filter(|p| p.is_some()).count();
    if count > 0 {
        eprintln!("Saved {count} file(s) to {}", dir.display());
    }
    Ok(paths)
}

fn prompt_for_output_dir(
    results: &[topk_rs::proto::v1::ctx::SearchResult],
) -> Result<Option<PathBuf>, Error> {
    let non_text_ids: Vec<String> = results
        .iter()
        .enumerate()
        .filter(|(_, r)| has_saveable_content(r))
        .map(|(i, _)| (i + 1).to_string())
        .collect();

    if non_text_ids.is_empty()
        || !std::io::stdin().is_terminal()
        || !std::io::stderr().is_terminal()
    {
        return Ok(None);
    }

    let ids_str = non_text_ids
        .iter()
        .map(|id| format!("{BLUE}[{id}]{RESET}"))
        .collect::<Vec<_>>()
        .join(", ");
    eprint!(
        "\n{BOLD}References:{RESET} {ids_str} contain non-text citations. Save to directory {DIM}[enter path or press Enter to skip]{RESET}: "
    );
    std::io::stderr().flush().ok();

    let mut input = String::new();
    std::io::stdin().lock().read_line(&mut input)?;

    eprint!("\x1b[2A\x1b[0J");
    std::io::stderr().flush().ok();

    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(trimmed)))
    }
}

fn rendered_display_line_count(rendered: &str) -> usize {
    let width = terminal_size()
        .map(|(TermWidth(w), _)| usize::from(w))
        .filter(|w| *w > 0)
        .unwrap_or(usize::MAX);

    rendered
        .lines()
        .map(|line| {
            let visible_len = visible_text_len(line);
            if width == usize::MAX {
                1
            } else {
                visible_len.max(1).div_ceil(width)
            }
        })
        .sum::<usize>()
        .max(1)
}

fn replace_rendered_stdout(previous_lines: usize, rendered: &str) -> Result<(), Error> {
    if !std::io::stdout().is_terminal() || previous_lines == 0 {
        println!("{rendered}");
        return Ok(());
    }

    print!("\x1b[{}A\x1b[0J", previous_lines);
    println!("{rendered}");
    std::io::stdout()
        .flush()
        .map_err(|e| Error::Internal(e.to_string()))
}

fn visible_text_len(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut i = 0usize;
    let mut visible = 0usize;

    while i < bytes.len() {
        if bytes[i] == 0x1b {
            i += 1;
            if i < bytes.len() && bytes[i] == b'[' {
                i += 1;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (b as char).is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }

        if let Some(ch) = line[i..].chars().next() {
            visible += 1;
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    visible
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
            .args(["-o", "json", "search", "summarize", "--dataset", &dataset])
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
    #[ignore]
    async fn search_returns_metadata_fields(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("meta-fields");

        cmd()
            .args(["dataset", "create", &dataset])
            .output()
            .unwrap();

        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/markdown.md");
        let out = cmd()
            .args([
                "-o",
                "json",
                "upload",
                "--dataset",
                &dataset,
                "--id",
                "meta-fields-doc",
                "--meta",
                r#"{"title": "My Test Document", "author": "Test Author"}"#,
                "--wait",
                "-y",
                file,
            ])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "upload failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );

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
