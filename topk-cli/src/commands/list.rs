use bytesize::ByteSize;
use comfy_table::{
    presets, Attribute, Cell, CellAlignment, Color, ColumnConstraint, ContentArrangement, Table,
    Width,
};
use terminal_size::{terminal_size, Width as TermWidth};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use topk_rs::{proto::v1::ctx::ListEntry, Client, Error};

use crate::output::{Output, RenderForHuman};

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

#[derive(Serialize, Deserialize)]
pub struct ListResult {
    pub entries: Vec<ListEntryRow>,
}

impl RenderForHuman for ListResult {
    fn render(&self) -> String {
        let rows = self
            .entries
            .iter()
            .map(|entry| {
                vec![
                    entry.id.clone(),
                    entry.name.clone(),
                    ByteSize(entry.size).to_string(),
                    entry.mime_type.clone(),
                ]
            })
            .collect::<Vec<_>>();

        render_human_table(&rows)
    }
}

fn render_human_table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "No documents.".to_string();
    }

    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(terminal_size().map(|(TermWidth(w), _)| w).unwrap_or(80))
        .set_header(["ID", "NAME", "SIZE", "TYPE"].into_iter().map(|header| {
            Cell::new(header)
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan)
        }))
        .set_constraints([
            ColumnConstraint::LowerBoundary(Width::Fixed(10)),
            ColumnConstraint::LowerBoundary(Width::Fixed(10)),
            ColumnConstraint::ContentWidth,
            ColumnConstraint::ContentWidth,
        ]);

    if let Some(column) = table.column_mut(2) {
        column.set_cell_alignment(CellAlignment::Right);
    }

    for row in rows {
        table.add_row([
            Cell::new(&row[0]),
            Cell::new(&row[1]),
            Cell::new(&row[2]),
            Cell::new(&row[3]).add_attribute(Attribute::Dim),
        ]);
    }

    table.to_string()
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
        let mut entries = Vec::new();
        while let Some(entry) = stream.next().await {
            let row: ListEntryRow = entry?
                .try_into()
                .map_err(|e: serde_json::Error| Error::Internal(e.to_string()))?;
            entries.push(row);
        }

        let count = entries.len() as u64;
        output
            .print(&ListResult { entries })
            .map_err(|e| Error::Internal(e.to_string()))?;
        return Ok(count);
    }

    let mut count: u64 = 0;
    while let Some(entry) = stream.next().await {
        let row: ListEntryRow = entry?
            .try_into()
            .map_err(|e: serde_json::Error| Error::Internal(e.to_string()))?;
        output
            .print_json_line(&row)
            .map_err(|e| Error::Internal(e.to_string()))?;
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{render_human_table, ListEntryRow};
    use crate::test_context::CliTestContext;
    use assert_cmd::Command;
    use regex::Regex;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

    fn strip_ansi(value: &str) -> String {
        Regex::new(r"\x1b\[[0-9;]*m")
            .unwrap()
            .replace_all(value, "")
            .into_owned()
    }

    #[test]
    fn human_table_keeps_later_columns_aligned_for_long_names() {
        let rendered = strip_ansi(&render_human_table(&[
            vec![
                "short-id".to_string(),
                "short.pdf".to_string(),
                "1.0 KB".to_string(),
                "application/pdf".to_string(),
            ],
            vec![
                "very-long-id".to_string(),
                "Biosimilar_and_Interchangeable_Products_The_US_FDA_Perspective.pdf".to_string(),
                "1.0 KB".to_string(),
                "application/pdf".to_string(),
            ],
        ]));

        let lines: Vec<&str> = rendered.lines().collect();
        assert_eq!(lines.len(), 3);

        let size_col = lines[1].find("1.0 KB").unwrap();
        assert_eq!(size_col, lines[2].find("1.0 KB").unwrap());

        let type_col = lines[1].find("application/pdf").unwrap();
        assert_eq!(type_col, lines[2].find("application/pdf").unwrap());
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn list_returns_uploaded_documents(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("list");

        // Upload two files
        let out = cmd()
            .current_dir(TESTS_DIR)
            .args([
                "-o",
                "json",
                "upload",
                r"pdfko\.pdf|markdown\.md",
                "-d",
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

        // List and parse NDJSON
        let out = cmd()
            .args(["-o", "json", "list", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );

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
        cmd()
            .args(["dataset", "create", &dataset])
            .output()
            .unwrap();

        let out = cmd()
            .args(["-o", "json", "list", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty());
    }
}
