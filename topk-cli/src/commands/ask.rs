use std::collections::HashMap;

use serde::Serialize;
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
    Reason,
    DeepResearch,
}

impl From<Mode> for topk_rs::proto::v1::ctx::Mode {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Auto => topk_rs::proto::v1::ctx::Mode::Auto,
            Mode::Summarize => topk_rs::proto::v1::ctx::Mode::Summarize,
            Mode::Reason => topk_rs::proto::v1::ctx::Mode::Reason,
            Mode::DeepResearch => topk_rs::proto::v1::ctx::Mode::DeepResearch,
        }
    }
}

#[derive(Serialize)]
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
        let mut out = String::new();
        out.push('\n');

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
                    out.push_str(&format!(" {}{}{}", BLUE, refs_inline, RESET));
                }
                out.push('\n');
            }
        }

        if !self.refs.is_empty() {
            let mut sorted_refs: Vec<_> = self.refs.iter().collect();
            sorted_refs.sort_by_key(|(id, _)| {
                let parts: Vec<u32> = id.split('_').filter_map(|p| p.parse().ok()).collect();
                parts
            });
            out.push('\n');
            out.push_str(&format!("{}References:{}", BOLD, RESET));
            for (id, r) in sorted_refs {
                out.push('\n');
                out.push_str(&format!(
                    "{}[{}]{} {}\n       {}{} · {} · {}{}",
                    BLUE,
                    id,
                    RESET,
                    format_content_text(r.content.as_ref()),
                    DIM,
                    r.dataset,
                    r.doc_id,
                    r.doc_type,
                    RESET,
                ));
                out.push('\n');
            }
        }

        out
    }
}

/// `topk ask`
pub async fn run(
    client: &Client,
    query: String,
    sources: Vec<String>,
    mode: Option<Mode>,
    fields: Option<Vec<String>>,
    output: &Output,
) -> Result<AskResult, Error> {
    let spinner = output.spinner("Asking...");

    let mut stream = client
        .ask(query, sources, None, mode.map(|m| m.into()), fields)
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
