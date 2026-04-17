use std::collections::HashMap;
use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use topk_rs::{
    proto::v1::ctx::{
        ask_result::{self, Answer},
        Fact, SearchResult,
    },
    Client, Error,
};

use super::search::{has_saveable_content, render_search_result, write_result_content};
use crate::output::{Output, BLUE, BOLD, DIM, RESET};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Mode {
    Auto,
    Summarize,
    Research,
}

impl From<Mode> for topk_rs::proto::v1::ctx::Mode {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Auto => topk_rs::proto::v1::ctx::Mode::Auto,
            Mode::Summarize => topk_rs::proto::v1::ctx::Mode::Summarize,
            Mode::Research => topk_rs::proto::v1::ctx::Mode::Research,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AskResult {
    pub(crate) facts: Vec<Fact>,
    pub(crate) refs: HashMap<String, SearchResult>,
    #[serde(skip, default)]
    pub(crate) saved: HashMap<String, PathBuf>,
}

impl From<Answer> for AskResult {
    fn from(a: Answer) -> Self {
        Self {
            facts: a.facts,
            refs: a.refs,
            saved: HashMap::new(),
        }
    }
}

fn ref_sort_key(s: &str) -> Vec<u64> {
    s.split('_')
        .map(|p| p.parse().unwrap_or(u64::MAX))
        .collect()
}

fn render_facts_section(facts: &[Fact]) -> String {
    if facts.is_empty() {
        return "No answer found.".to_string();
    }
    let facts_text = facts
        .iter()
        .filter_map(|fact| {
            let refs_inline = if fact.ref_ids.is_empty() {
                None
            } else {
                let ids = fact
                    .ref_ids
                    .iter()
                    .map(|id| format!("[{id}]"))
                    .collect::<Vec<_>>()
                    .join(" ");
                Some(format!("{BLUE}{ids}{RESET}"))
            };
            let parts: Vec<&str> = [fact.fact.as_str()]
                .into_iter()
                .chain(refs_inline.as_deref())
                .filter(|s| !s.is_empty())
                .collect();
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" "))
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{BOLD}Facts:{RESET}\n{facts_text}")
}

fn render_refs_section(
    refs: &HashMap<String, SearchResult>,
    saved: &HashMap<String, PathBuf>,
) -> String {
    if refs.is_empty() {
        return String::new();
    }
    let mut sorted_refs: Vec<_> = refs.iter().collect();
    sorted_refs.sort_by(|(a, _), (b, _)| ref_sort_key(a).cmp(&ref_sort_key(b)));

    let ref_lines: Vec<String> = sorted_refs
        .into_iter()
        .map(|(id, r)| render_search_result(id, r, saved.get(id), Some(560)))
        .collect();

    format!("{BOLD}References:{RESET}\n{}", ref_lines.join("\n\n"))
}

fn save_refs(dir: &Path, refs: &HashMap<String, SearchResult>) -> Result<HashMap<String, PathBuf>, Error> {
    refs.iter()
        .filter_map(|(ref_id, r)| {
            match write_result_content(dir, ref_id, r) {
                Ok(Some(path)) => Some(Ok((ref_id.clone(), path))),
                Ok(None) => None,
                Err(err) => Some(Err(Error::IoError(err))),
            }
        })
        .collect()
}

/// `topk ask`
pub async fn run(
    client: &Client,
    query: String,
    datasets: Vec<String>,
    mode: Option<Mode>,
    fields: Option<Vec<String>>,
    output_dir: Option<PathBuf>,
    output: &Output,
) -> Result<(), Error> {
    let spinner = output.spinner("Asking...");

    let mut stream = client
        .ask(query, datasets, None, mode.map(|m| m.into()), fields)
        .await?
        .into_inner();

    let mut answer: Option<Answer> = None;

    while let Some(result) = stream.next().await {
        let result = result?;
        match result.message {
            Some(ask_result::Message::Reason(r)) => {
                spinner.println(format!("[thinking] {}", r.thought));
            }
            Some(ask_result::Message::Search(s)) => {
                spinner.println(format!("[searching] {}", s.objective));
                for fact in &s.facts {
                    spinner.println(format!(" - {}", fact.fact));
                }
            }
            Some(ask_result::Message::Answer(a)) => {
                answer = Some(a);
            }
            None => return Err(Error::InvalidProto),
        }
    }

    spinner.finish();

    let result: AskResult = answer
        .map(Into::into)
        .ok_or_else(|| Error::Internal("No answer found".to_string()))?;

    // JSON: save if output_dir given, then serialize
    if !output.is_human() {
        let mut result = result;
        if let Some(ref dir) = output_dir {
            result.saved = save_refs(dir, &result.refs)?;
        }
        output
            .print_json(&result)
            .map_err(|e| Error::Internal(e.to_string()))?;
        return Ok(());
    }

    // Human mode: print facts immediately
    println!("{}", render_facts_section(&result.facts));

    // Prompt for output dir if there are non-text refs and none was passed
    let output_dir = if output_dir.is_some() {
        output_dir
    } else {
        let mut non_text_ids: Vec<&str> = result
            .refs
            .iter()
            .filter(|(_, r)| has_saveable_content(r))
            .map(|(id, _)| id.as_str())
            .collect();
        non_text_ids.sort_by(|a, b| ref_sort_key(a).cmp(&ref_sort_key(b)));

        if !non_text_ids.is_empty()
            && std::io::stdin().is_terminal()
            && std::io::stderr().is_terminal()
        {
            let ids_str = non_text_ids
                .iter()
                .map(|id| format!("{BLUE}[{id}]{RESET}"))
                .collect::<Vec<_>>()
                .join(", ");
            eprint!("\n{BOLD}References:{RESET} {ids_str} contain non-text citations. Save to directory {DIM}[enter path or press Enter to skip]{RESET}: ");
            std::io::stderr().flush().ok();

            let mut input = String::new();
            std::io::stdin().lock().read_line(&mut input)?;

            // Clear the blank line + prompt+input line, leaving cursor where prompt started
            eprint!("\x1b[2A\x1b[0J");
            std::io::stderr().flush().ok();

            let trimmed = input.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            }
        } else {
            None
        }
    };

    // Save files and print refs
    let saved = if let Some(ref dir) = output_dir {
        save_refs(dir, &result.refs)?
    } else {
        HashMap::new()
    };

    let refs_text = render_refs_section(&result.refs, &saved);
    if !refs_text.is_empty() {
        println!("\n{refs_text}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::AskResult;
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn ask_returns_result(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("test");
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

        let out = cmd()
            .args(["-o", "json", "ask", "summarize", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

        let _: AskResult = serde_json::from_slice(&out.stdout).unwrap();
    }
}
