use serde::Serialize;
use topk_rs::{Client, Error};

use crate::output::RenderForHuman;
use crate::util::confirm;

#[derive(Serialize)]
pub struct DeleteResult {
    deleted: bool,
    skipped: Option<bool>,
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

pub async fn run(client: &Client, dataset: &str, doc_id: impl Into<String>, yes: bool) -> Result<DeleteResult, Error> {
    let doc_id = doc_id.into();
    if !yes && !confirm(&format!("Delete document '{}'? [y/N] ", doc_id))? {
        return Ok(DeleteResult { deleted: false, skipped: Some(true) });
    }
    client.dataset(dataset).delete(doc_id).await?;
    Ok(DeleteResult { deleted: true, skipped: None })
}
