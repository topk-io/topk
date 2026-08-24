use std::fmt;

use serde::{Deserialize, Serialize};
use topk_rs::{Client, Error};

use crate::output::Output;

#[derive(Serialize, Deserialize)]
pub struct DeleteResult {
    pub handle: Option<String>,
}

impl fmt::Display for DeleteResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(handle) = &self.handle {
            f.write_str(&format!("Deleting document... (handle: {handle})"))
        } else {
            f.write_str("Delete skipped.")
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct DeleteArgs {
    /// Dataset name
    #[arg(short = 'd', long, value_name = "DATASET_NAME")]
    pub dataset: String,
    /// Document ID
    #[arg(long)]
    pub id: String,
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}

/// `topk delete`
pub async fn run(
    client: &Client,
    args: &DeleteArgs,
    output: &Output,
) -> Result<DeleteResult, Error> {
    if !output.confirm_or_yes(&format!("Delete document '{}'? ", args.id), args.yes)? {
        return Ok(DeleteResult { handle: None });
    }

    let handle = client
        .dataset(&args.dataset)
        .delete(args.id.clone())
        .await?;

    Ok(DeleteResult {
        handle: Some(handle),
    })
}
