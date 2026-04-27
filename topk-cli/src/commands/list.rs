use std::fmt;

use bytesize::ByteSize;
use colored::Colorize;
use comfy_table::{
    presets, Attribute, Cell, CellAlignment, Color, ColumnConstraint, ContentArrangement, Table,
    Width,
};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use terminal_size::{terminal_size, Width as TermWidth};
use topk_rs::{Client, Error};

#[derive(Serialize, Deserialize)]
pub struct ListEntry {
    pub id: String,
    pub name: String,
    pub size: ByteSize,
    pub mime_type: String,
    pub status: String,
    pub status_reason: Option<String>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl From<topk_rs::proto::v1::ctx::ListEntry> for ListEntry {
    fn from(entry: topk_rs::proto::v1::ctx::ListEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            size: ByteSize::b(entry.size),
            mime_type: entry.mime_type,
            status: entry.status,
            status_reason: entry.status_reason,
            metadata: entry
                .metadata
                .into_iter()
                .map(|(k, v)| (k, serde_json::to_value(v).unwrap_or_default()))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListResult {
    pub entries: Vec<ListEntry>,
}

fn render_status(status: &str, reason: Option<&str>) -> String {
    let label = |base: &str| match reason {
        Some(r) => format!("{base} ({r})"),
        None => base.to_string(),
    };
    match status {
        "pending" => label("Pending").yellow().to_string(),
        "ready" => label("Ready").green().to_string(),
        "error" => label("Error").red().to_string(),
        _ => label(status),
    }
}

fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    let len = value.chars().count();
    if len <= max_chars {
        return value.to_string();
    }

    if max_chars <= 1 {
        return "…".to_string();
    }

    format!("{}…", value.chars().take(max_chars - 1).collect::<String>())
}

fn terminal_width() -> u16 {
    terminal_size().map(|(TermWidth(w), _)| w).unwrap_or(80)
}

fn render_full_table(entries: &[ListEntry], width: u16) -> String {
    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(width)
        .set_header(
            ["NAME", "ID", "STATUS", "SIZE", "TYPE"]
                .into_iter()
                .map(|header| {
                    Cell::new(header)
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Cyan)
                }),
        )
        .set_constraints([
            ColumnConstraint::LowerBoundary(Width::Fixed(28)),
            ColumnConstraint::LowerBoundary(Width::Fixed(16)),
            ColumnConstraint::ContentWidth,
            ColumnConstraint::ContentWidth,
            ColumnConstraint::ContentWidth,
        ]);

    if let Some(column) = table.column_mut(3) {
        column.set_cell_alignment(CellAlignment::Right);
    }

    for entry in entries {
        table.add_row([
            Cell::new(&entry.name),
            Cell::new(&entry.id),
            Cell::new(render_status(&entry.status, entry.status_reason.as_deref())),
            Cell::new(entry.size.to_string()),
            Cell::new(&entry.mime_type).add_attribute(Attribute::Dim),
        ]);
    }

    table.to_string()
}

fn render_compact_table(entries: &[ListEntry], width: u16) -> String {
    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(width)
        .set_header(["NAME", "ID", "STATUS", "SIZE"].into_iter().map(|header| {
            Cell::new(header)
                .add_attribute(Attribute::Bold)
                .fg(Color::Cyan)
        }))
        .set_constraints([
            ColumnConstraint::LowerBoundary(Width::Fixed(36)),
            ColumnConstraint::LowerBoundary(Width::Fixed(16)),
            ColumnConstraint::ContentWidth,
            ColumnConstraint::ContentWidth,
        ]);

    if let Some(column) = table.column_mut(3) {
        column.set_cell_alignment(CellAlignment::Right);
    }

    for entry in entries {
        table.add_row([
            Cell::new(truncate_with_ellipsis(&entry.name, 40)),
            Cell::new(truncate_with_ellipsis(&entry.id, 16)),
            Cell::new(render_status(&entry.status, entry.status_reason.as_deref())),
            Cell::new(entry.size.to_string()),
        ]);
    }

    table.to_string()
}

fn render_stacked_entries(entries: &[ListEntry]) -> String {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}\n  id: {}\n  status: {}\n  size: {}\n  type: {}",
                entry.name,
                entry.id,
                render_status(&entry.status, entry.status_reason.as_deref()),
                entry.size.to_string(),
                entry.mime_type
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

impl fmt::Display for ListResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.entries.is_empty() {
            return f.write_str("No documents.");
        }

        let rendered = match terminal_width() {
            width if width >= 120 => render_full_table(&self.entries, width),
            width if width >= 80 => render_compact_table(&self.entries, width),
            _ => render_stacked_entries(&self.entries),
        };

        f.write_str(&rendered)
    }
}

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Dataset to list documents from
    #[arg(short = 'd', long, value_name = "DATASET_NAME")]
    pub dataset: String,
    /// Metadata fields to include (repeatable)
    #[arg(short = 'f', long = "field")]
    pub fields: Option<Vec<String>>,
}

/// `topk list`
pub async fn run(
    client: &Client,
    args: &ListArgs,
) -> Result<impl Stream<Item = Result<ListEntry, Error>>, Error> {
    Ok(client
        .dataset(&args.dataset)
        .list(args.fields.clone(), None)
        .await?
        .into_inner()
        .map(|entry| entry.map_err(Error::from).map(ListEntry::from)))
}

#[cfg(test)]
mod tests {
    use super::ListEntry;
    use crate::commands::test_context::CliTestContext;
    use assert_cmd::Command;
    use bytesize::ByteSize;
    use test_context::test_context;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    const TESTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests");

    #[test_context(CliTestContext)]
    #[tokio::test]
    #[ignore]
    async fn list_returns_uploaded_documents(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("list");
        ctx.create_dataset(&dataset);

        for pattern in ["pdfko.pdf", "markdown.md"] {
            let out = cmd()
                .current_dir(TESTS_DIR)
                .args([
                    "-o", "json", "upload", pattern, "-d", &dataset, "-y", "--wait",
                ])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }

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

        let entries: Vec<ListEntry> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.size > ByteSize::b(0)));
    }

    #[test_context(CliTestContext)]
    #[tokio::test]
    async fn list_empty_dataset(ctx: &mut CliTestContext) {
        let dataset = ctx.wrap("list-empty");
        ctx.create_dataset(&dataset);

        let out = cmd()
            .args(["-o", "json", "list", "--dataset", &dataset])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let entries: Vec<ListEntry> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(entries.is_empty());
    }
}
