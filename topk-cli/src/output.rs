use std::io::Write;

use anyhow::Result;
use clap::ValueEnum;
use comfy_table::presets::{NOTHING, UTF8_FULL};
use comfy_table::{ContentArrangement, Row, Table};
use serde::Serialize;

const MIN_WIDTH: u16 = 20;

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum Output {
    Text,
    Json,
}

pub trait Tabular: Serialize {
    /// Column labels for this resource in display order.
    fn columns(&self) -> Vec<(&'static str, String)>;
}

/// One JSON object per line.
pub fn json_line(out: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)?;
    Ok(())
}

/// Print a list of resources or a single resource.
/// JSON: one JSON object per line.
/// Text: a single resource prints as `label: value` rows, several print as a table
pub fn print(
    out: &mut impl Write,
    output: Output,
    items: impl IntoIterator<Item = impl Tabular>,
) -> Result<()> {
    if output == Output::Json {
        for item in items {
            json_line(out, &item)?;
        }
        return Ok(());
    }
    let mut rows: Vec<_> = items.into_iter().map(|item| item.columns()).collect();
    let table = match rows.as_mut_slice() {
        [] => return Ok(()),
        [row] => {
            let mut table = Table::new();
            table.load_preset(NOTHING);
            for (label, value) in row.drain(..) {
                table.add_row([format!("{label}:"), value]);
            }
            table
        }
        [first, ..] => {
            let mut table = self::table(first.iter().map(|(label, _)| *label));
            table.add_rows(
                rows.into_iter()
                    .map(|row| row.into_iter().map(|(_, value)| value)),
            );
            table
        }
    };
    writeln!(out, "{table}")?;
    Ok(())
}

pub fn table(header: impl Into<Row>) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(header);
    if table.width().is_some_and(|width| width >= MIN_WIDTH) {
        table.set_content_arrangement(ContentArrangement::Dynamic);
    }
    table
}
