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

use crate::output::Output;
use crate::util::resolve_query;

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
    pub refs: HashMap<String, SearchResult>,
}

impl From<Answer> for AskResult {
    fn from(a: Answer) -> Self {
        Self {
            facts: a.facts,
            refs: a.refs.into_iter().collect(),
        }
    }
}

impl fmt::Display for AskResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&render_facts(&self.facts))
    }
}

fn render_facts(facts: &[Fact]) -> String {
    if facts.is_empty() {
        return "No answer found.".to_string();
    }

    let facts_text = facts
        .iter()
        .map(|fact| {
            if fact.ref_ids.is_empty() {
                fact.fact.clone()
            } else {
                let refs = fact
                    .ref_ids
                    .iter()
                    .map(|id| format!("[{id}]"))
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{} {}", fact.fact, refs.blue())
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    format!("{}\n{}", "Facts:".bold(), facts_text)
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
    let query = resolve_query(args.query.clone())?;

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

    Ok(answer.into())
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
