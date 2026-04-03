use anyhow::Result;
use futures::TryStreamExt;
use serde::Serialize;
use topk_rs::{
    proto::v1::ctx::{content, Content, SearchResult},
    Client,
};

use crate::output::{table, RenderForHuman};

#[derive(Serialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
}

impl RenderForHuman for SearchResults {
    fn render(&self) -> String {
        if self.results.is_empty() {
            return "No results.".to_string();
        }
        table(
            vec!["DOC ID", "DATASET", "TYPE", "CONTENT"],
            self.results
                .iter()
                .map(|r| vec![
                    r.doc_id.clone(),
                    r.dataset.clone(),
                    r.doc_type.clone(),
                    format_content_text(r.content.as_ref()),
                ])
                .collect(),
        )
    }
}

pub async fn run(
    client: &Client,
    query: String,
    sources: Vec<String>,
    top_k: u32,
    fields: Vec<String>,
) -> Result<SearchResults> {
    let stream = client
        .search(query, sources, top_k, None, fields)
        .await?
        .into_inner();

    let results = stream.try_collect().await?;
    Ok(SearchResults { results })
}

fn format_content_text(content: Option<&Content>) -> String {
    match content.and_then(|c| c.data.as_ref()) {
        Some(content::Data::Chunk(chunk)) => {
            let text = if chunk.text.len() > 200 {
                format!("{}...", &chunk.text[..200])
            } else {
                chunk.text.clone()
            };
            if chunk.doc_pages.is_empty() {
                text
            } else {
                let pages: Vec<String> = chunk.doc_pages.iter().map(|p| p.to_string()).collect();
                format!("{} [p.{}]", text, pages.join(","))
            }
        }
        Some(content::Data::Page(page)) => format!("<page {}>", page.page_number),
        Some(content::Data::Image(img)) => format!("<image {} {}>", img.mime_type, bytesize::ByteSize(img.data.len() as u64)),
        None => String::new(),
    }
}

