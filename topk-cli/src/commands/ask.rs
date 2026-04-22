use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use colored::Colorize;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use topk_rs::{
    proto::v1::ctx::{
        ask_result::{self, Answer},
        Fact,
    },
    Client, Error,
};

use super::search::{render_search_result, write_result_content, SearchResult};
use crate::output::Output;
use crate::util::resolve_query;
use topk_rs::proto::v1::ctx::content;

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
}

impl From<Answer> for AskResult {
    fn from(a: Answer) -> Self {
        Self {
            facts: a.facts,
            refs: a
                .refs
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        SearchResult {
                            result: v,
                            path: None,
                        },
                    )
                })
                .collect(),
        }
    }
}

impl fmt::Display for AskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let facts = render_facts(&self.facts);
        let refs = render_refs(&self.refs)
            .map(|r| format!("\n{r}"))
            .unwrap_or_default();
        f.write_str(&format!("{facts}{refs}"))
    }
}

fn has_non_text_search_results(
    refs: &HashMap<String, topk_rs::proto::v1::ctx::SearchResult>,
) -> bool {
    refs.values().any(|r| {
        !matches!(
            r.content.as_ref().and_then(|c| c.data.as_ref()),
            Some(content::Data::Chunk(_)) | None
        )
    })
}

fn render_facts(facts: &[Fact]) -> String {
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
                Some(format!("{}", ids.blue()))
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
    format!("{}\n{facts_text}", "Facts:".bold())
}

fn render_refs(refs: &HashMap<String, SearchResult>) -> Option<String> {
    if refs.is_empty() {
        return None;
    }
    let ref_lines: Vec<String> = refs
        .iter()
        .map(|(id, r)| render_search_result(id, r, Some(560)))
        .collect();

    Some(format!(
        "{}\n{}",
        "References:".bold(),
        ref_lines.join("\n\n")
    ))
}

fn write_results(answer: Answer, output_dir: Option<&Path>) -> Result<AskResult, Error> {
    let refs = answer
        .refs
        .into_iter()
        .map(|(k, v)| {
            let path = output_dir
                .map(|dir| write_result_content(dir, &k, &v))
                .transpose()
                .map_err(Error::IoError)?
                .flatten();
            Ok::<_, Error>((k, SearchResult { result: v, path }))
        })
        .collect::<Result<_, _>>()?;

    Ok(AskResult {
        facts: answer.facts,
        refs,
    })
}

#[derive(Debug, clap::Args)]
pub struct AskArgs {
    /// Question to ask (reads from stdin if omitted)
    pub query: Option<String>,
    /// Dataset to search (repeatable)
    #[arg(short = 'd', long = "dataset")]
    pub datasets: Vec<String>,
    /// Query mode
    #[arg(short = 'm', long)]
    pub mode: Option<Mode>,
    /// Metadata fields to include in results (repeatable)
    #[arg(short = 'f', long = "field")]
    pub fields: Option<Vec<String>>,
    /// Save search result content (images, text chunks) to a directory
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<PathBuf>,
}

/// `topk ask`
pub async fn run(client: &Client, args: &AskArgs, output: &Output) -> Result<AskResult, Error> {
    let query = resolve_query(args.query.clone())
        .map_err(|e| Error::Input(anyhow::anyhow!(e)))?
        .ok_or_else(|| {
            Error::Input(anyhow::anyhow!(
                "query is required; pass it as an argument or pipe it via stdin"
            ))
        })?;

    let spinner = output.spinner("Asking...");

    let mut stream = client
        .ask(
            query,
            args.datasets.clone(),
            None,
            args.mode.clone().map(Into::into),
            args.fields.clone(),
        )
        .await?
        .into_inner();

    let mut answer: Option<Answer> = None;
    while let Some(item) = stream.next().await {
        let item = item?;
        match item.message {
            Some(ask_result::Message::Reason(r)) => {
                spinner.print(format!("[thinking] {}", r.thought));
            }
            Some(ask_result::Message::Search(s)) => {
                spinner.print(format!("[searching] {}", s.objective));
                for fact in &s.facts {
                    spinner.print(format!(" - {}", fact.fact));
                }
            }
            Some(ask_result::Message::Answer(a)) => {
                answer = Some(a);
            }
            None => return Err(Error::InvalidProto),
        }
    }

    spinner.finish();

    let answer = answer.ok_or_else(|| Error::Internal("No answer found".to_string()))?;

    let output_dir = match &args.output_dir {
        Some(dir) => Some(dir.clone()),
        None if !output.is_json() && has_non_text_search_results(&answer.refs) => {
            let ids = answer
                .refs
                .iter()
                .filter(|(_, r)| {
                    !matches!(
                        r.content.as_ref().and_then(|c| c.data.as_ref()),
                        Some(content::Data::Chunk(_)) | None
                    )
                })
                .map(|(id, _)| format!("{}", format!("[{id}]").blue()))
                .collect::<Vec<_>>()
                .join(", ");
            output.prompt_dir(format!(
                "{ids} contain non-text citations. Save to directory (or Enter to skip)"
            ))?
        }
        None => None,
    };

    write_results(answer, output_dir.as_deref())
}

#[cfg(test)]
mod tests {
    use super::AskResult;
    use crate::commands::test_context::{CliTestContext, OutputJsonExt};
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

        let _: AskResult = out.json().unwrap();
    }
}
