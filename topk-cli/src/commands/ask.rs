use std::collections::HashMap;

use serde::Serialize;
use tokio_stream::StreamExt;
use topk_rs::{
    Client, Error, proto::v1::ctx::{Fact, Mode, SearchResult, ask_result::{self, Answer}, content}
};

use crate::output::{Output, RenderForHuman};

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum AskMode {
    Auto,
    Summarize,
    Reason,
    DeepResearch,
}

impl From<AskMode> for Mode {
    fn from(m: AskMode) -> Self {
        match m {
            AskMode::Auto => Mode::Auto,
            AskMode::Summarize => Mode::Summarize,
            AskMode::Reason => Mode::Reason,
            AskMode::DeepResearch => Mode::DeepResearch,
        }
    }
}

#[derive(Serialize)]
pub struct AskResult {
    facts: Vec<Fact>,
    refs: HashMap<String, SearchResult>,
}

impl From<Answer> for AskResult {
    fn from(a: Answer) -> Self {
        Self { facts: a.facts, refs: a.refs }
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
                if fact.ref_ids.is_empty() {
                    out.push_str(&fact.fact);
                } else {
                    let refs_inline: Vec<String> =
                        fact.ref_ids.iter().map(|id| format!("[{}]", id)).collect();
                    out.push_str(&format!("{} {}", fact.fact, refs_inline.join(", ")));
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
            out.push_str("References:\n");
            for (id, r) in sorted_refs {
                let filename = r.doc_id.rsplit('/').next().unwrap_or(&r.doc_id);
                out.push('\n');
                match r.content.as_ref().and_then(|c| c.data.as_ref()) {
                    Some(content::Data::Chunk(chunk)) => {
                        let pages: Vec<String> = chunk.doc_pages.iter().map(|p| p.to_string()).collect();
                        let location = match pages.as_slice() {
                            [] => String::new(),
                            [page] => format!(" · page {}", page),
                            pages => format!(" · pages {}", pages.join(", ")),
                        };
                        out.push_str(&format!("[{}] {}{}\n", id, filename, location));
                        out.push_str(&chunk.text);
                        out.push('\n');
                    }
                    Some(content::Data::Page(page)) => {
                        out.push_str(&format!("[{}] {} · page {}\n", id, filename, page.page_number));
                    }
                    Some(content::Data::Image(img)) => {
                        out.push_str(&format!("[{}] {} · {}\n", id, filename, img.mime_type));
                    }
                    None => {
                        out.push_str(&format!("[{}] {}\n", id, filename));
                    }
                }
            }
        }

        out
    }
}

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
        .ask(query, sources, None, mode, fields)
        .await?
        .into_inner();

    let mut answer: Option<Answer> = None;

    while let Some(result) = stream.next().await {
        let result = result?;
        match result.message {
            Some(ask_result::Message::Reason(r)) => {
                spinner.println(format!("[thinking] {}", r.thought));
            }
            Some(ask_result::Message::Search(s_msg)) => {
                spinner.println(format!("[searching] {}", s_msg.objective));
                for fact in &s_msg.facts {
                    spinner.println(format!("  - {}", fact.fact));
                }
            }
            Some(ask_result::Message::Answer(a)) => {
                answer = Some(a);
            }
            None => return Err(Error::InvalidProto),
        }
    }

    spinner.finish();

    answer.map(Into::into).ok_or_else(|| Error::Internal("No answer found".to_string()))
}

