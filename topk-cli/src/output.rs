use std::io::Write;

use anyhow::Result;
use clap::ValueEnum;
use comfy_table::presets::{NOTHING, UTF8_FULL};
use comfy_table::{ContentArrangement, Table};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, ValueEnum)]
pub enum Output {
    Text,
    Json,
}

pub trait Tabular: Serialize {
    /// Column labels for this resource in display order.
    fn columns(&self) -> Vec<(&'static str, String)>;
}

#[derive(Serialize)]
pub struct Selected<T> {
    #[serde(flatten)]
    pub item: T,
    pub selected: bool,
}

impl<T: Tabular> Tabular for Selected<T> {
    fn columns(&self) -> Vec<(&'static str, String)> {
        let mut columns = self.item.columns();
        if self.selected {
            if let Some((_, name)) = columns.first_mut() {
                name.push('*');
            }
        }
        columns
    }
}

/// One JSON object per line.
pub fn json_line(out: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *out, value)?;
    writeln!(out)?;
    Ok(())
}

/// Print a single resource as labeled values, or one JSON object.
pub fn print(out: &mut impl Write, output: Output, item: impl Tabular) -> Result<()> {
    if output == Output::Json {
        return json_line(out, &item);
    }
    let mut table = Table::new();
    table.load_preset(NOTHING);
    for (label, value) in item.columns() {
        table.add_row([format!("{label}:"), value]);
    }
    writeln!(out, "{table}")?;
    Ok(())
}

/// Print a list as a table even with one result, or one JSON object per line.
pub fn print_list(
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
    let mut rows = items.into_iter().map(|item| item.columns());
    let Some(first) = rows.next() else {
        return Ok(());
    };
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(first.iter().map(|(label, _)| *label));
    table.add_rows(
        std::iter::once(first)
            .chain(rows)
            .map(|row| row.into_iter().map(|(_, value)| value)),
    );
    writeln!(out, "{table}")?;
    Ok(())
}
