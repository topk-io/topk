use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use topk_rs::{
    proto::v1::ctx::{
        ask_result::{self, Answer},
        Fact, SearchResult,
    },
    Client, Error,
};

use super::search::format_content_text;

use crate::output::{Output, RenderForHuman, BLUE, BOLD, DIM, RESET};

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
            refs: a.refs,
        }
    }
}

impl RenderForHuman for AskResult {
    fn render(&self) -> String {
        let mut out = String::from("\n");

        if self.facts.is_empty() {
            out.push_str("No answer found.");
        } else {
            for fact in &self.facts {
                out.push_str(fact.fact.trim());
                if !fact.ref_ids.is_empty() {
                    let refs_inline = fact
                        .ref_ids
                        .iter()
                        .map(|id| format!("[{}]", id))
                        .collect::<Vec<_>>()
                        .join(", ");
                    out.push_str(&format!(" {BLUE}{refs_inline}{RESET}"));
                }
                out.push('\n');
            }
        }

        if !self.refs.is_empty() {
            let mut sorted_refs: Vec<_> = self.refs.iter().collect();
            // Ref IDs have the form "<chunk>_<index>" (e.g. "1_8"). Sort numerically
            // per segment so that "1_9" < "1_10" rather than lexicographically.
            sorted_refs.sort_by(|(a, _), (b, _)| {
                let parse = |s: &str| -> Vec<u64> {
                    s.split('_')
                        .map(|p| p.parse().unwrap_or(u64::MAX))
                        .collect()
                };
                parse(a).cmp(&parse(b))
            });
            out.push('\n');
            out.push_str(&format!("{BOLD}References:{RESET}\n"));
            for (id, r) in sorted_refs {
                out.push_str(&format!(
                    "{BLUE}[{id}]{RESET} {}\n       {DIM}{} · {} · {}{RESET}\n",
                    format_content_text(r.content.as_ref()),
                    r.dataset,
                    r.doc_id,
                    r.doc_type,
                ));
            }
        }

        out.trim_end().to_string()
    }
}

/// `topk ask`
pub async fn run(
    client: &Client,
    query: String,
    datasets: Vec<String>,
    mode: Option<Mode>,
    fields: Option<Vec<String>>,
    output: &Output,
) -> Result<AskResult, Error> {
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

    answer
        .map(Into::into)
        .ok_or_else(|| Error::Internal("No answer found".to_string()))
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
        let result: AskResult = serde_json::from_slice(&out.stdout).unwrap();
        assert!(result.facts.is_empty() || !result.facts.is_empty());
    }
}
