use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use colored::Colorize;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use topk_rs::{
    proto::v1::ctx::{
        ask_result::{self, Answer},
        Fact, SearchResult,
    },
    Client, Error,
};

use crate::util::read_query_from_stdin;
use crate::{commands::search::render_search_result, output::Output};

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
    pub facts: Vec<Fact>,
    pub refs: HashMap<String, SearchResult>,

    #[serde(skip)]
    pub(crate) show_refs: bool,
}

impl AskResult {
    fn from_answer(a: Answer, show_refs: bool) -> Self {
        Self {
            facts: a.facts,
            refs: a.refs.into_iter().collect(),
            show_refs,
        }
    }

    pub fn render_refs(&self, paths: &HashMap<String, PathBuf>) -> Option<String> {
        if !self.show_refs || self.refs.is_empty() {
            return None;
        }

        let refs_text = self
            .refs
            .iter()
            .map(|(ref_id, result)| {
                render_search_result(ref_id, result, paths.get(ref_id).map(PathBuf::as_path))
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        if refs_text.is_empty() {
            None
        } else {
            Some(format!("\n{}\n{refs_text}", "References:".bold()))
        }
    }
}

impl fmt::Display for AskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.facts.is_empty() {
            return f.write_str("No answer found.");
        }

        let text = self
            .facts
            .iter()
            .map(|fact| {
                if self.show_refs && !fact.ref_ids.is_empty() {
                    let refs = fact
                        .ref_ids
                        .iter()
                        .map(|id| format!("[{id}]"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("{} {}", fact.fact, refs.blue())
                } else {
                    fact.fact.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        f.write_str(&text)
    }
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
    /// Show citations in the answer
    #[arg(long, default_value = "false")]
    pub show_refs: bool,
    /// Save search result content (images, text chunks) to a directory
    #[arg(long, value_name = "DIR")]
    pub output_dir: Option<PathBuf>,
}

/// `topk ask`
pub async fn run(client: &Client, args: &AskArgs, output: &Output) -> Result<AskResult, Error> {
    let query = match args.query.clone() {
        Some(query) => query,
        None => read_query_from_stdin()?,
    };

    let spinner = output.spinner("Answering...");

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

    Ok(AskResult::from_answer(answer, args.show_refs))
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
