use bytesize::ByteSize;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use topk_rs::{proto::v1::ctx::ListEntry, Client, Error};

use crate::output::{Output, BOLD, DIM, RESET};

#[derive(Serialize, Deserialize)]
pub struct ListEntryRow {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl TryFrom<ListEntry> for ListEntryRow {
    type Error = serde_json::Error;

    fn try_from(e: ListEntry) -> Result<Self, serde_json::Error> {
        let metadata = e
            .metadata
            .into_iter()
            .map(|(k, v)| serde_json::to_value(v).map(|v| (k, v)))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            id: e.id,
            name: e.name,
            size: e.size,
            mime_type: e.mime_type,
            metadata,
        })
    }
}

/// `topk list`
pub async fn run(
    client: &Client,
    dataset: &str,
    fields: Option<Vec<String>>,
    output: &Output,
) -> Result<u64, Error> {
    let mut stream = client
        .dataset(dataset)
        .list(fields, None)
        .await?
        .into_inner();

    if output.is_human() {
        println!("{BOLD}{:<60}  {:<40}  {:>10}  {}{RESET}", "ID", "NAME", "SIZE", "TYPE");

        let mut count: u64 = 0;
        while let Some(entry) = stream.next().await {
            let entry = entry?;
            println!(
                "{:<60}  {:<40}  {:>10}  {DIM}{}{RESET}",
                entry.id,
                entry.name,
                ByteSize(entry.size).to_string(),
                entry.mime_type,
            );
            count += 1;
        }

        if count == 0 {
            println!("No documents.");
        }

        Ok(count)
    } else {
        let mut count: u64 = 0;
        while let Some(entry) = stream.next().await {
            let row: ListEntryRow = entry?
                .try_into()
                .map_err(|e: serde_json::Error| Error::Internal(e.to_string()))?;
            println!(
                "{}",
                serde_json::to_string(&row).map_err(|e| Error::Internal(e.to_string()))?
            );
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::ListEntryRow;
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn list_returns_uploaded_documents(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("list");

        // Upload two files
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args(["-o", "json", "upload", r"pdfko\.pdf|markdown\.md", "-d", &dataset, "-y", "--wait"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

        // List and parse NDJSON
        let out = cmd()
            .args(["-o", "json", "list", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

        let entries: Vec<ListEntryRow> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.size > 0));
        assert!(entries.iter().all(|e| !e.mime_type.is_empty()));
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn list_empty_dataset(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("list-empty");
        cmd().args(["dataset", "create", &dataset]).output().unwrap();

        let out = cmd()
            .args(["-o", "json", "list", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        assert!(out.stdout.is_empty());
    }
}
