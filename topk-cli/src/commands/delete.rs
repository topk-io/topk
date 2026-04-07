use serde::Serialize;
use topk_rs::{Client, Error};

use crate::output::{Output, RenderForHuman};

#[derive(Serialize)]
pub struct DeleteResult {
    deleted: bool,
    skipped: Option<bool>,
    handle: Option<String>,
}

impl RenderForHuman for DeleteResult {
    fn render(&self) -> String {
        if self.skipped == Some(true) {
            "Deletion skipped.".to_string()
        } else {
            "Document deleted.".to_string()
        }
    }
}

/// `topk delete`
pub async fn run(
    client: &Client,
    dataset: &str,
    doc_id: impl Into<String>,
    yes: bool,
    output: &Output,
) -> Result<DeleteResult, Error> {
    let doc_id = doc_id.into();

    if !yes && !output.confirm(&format!("Delete document '{}'? [y/N] ", doc_id))? {
        return Ok(DeleteResult {
            deleted: false,
            skipped: Some(true),
            handle: None,
        });
    }

    let result = client.dataset(dataset).delete(doc_id).await?;

    Ok(DeleteResult {
        deleted: true,
        skipped: None,
        handle: Some(result.into_inner().handle),
    })
}
