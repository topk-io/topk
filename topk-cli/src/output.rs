use std::io::Write;

use anyhow::Result;
use comfy_table::presets::NOTHING;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use serde::Serialize;

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
    json: bool,
    items: impl IntoIterator<Item = impl Tabular>,
) -> Result<()> {
    if json {
        for item in items {
            json_line(out, &item)?;
        }
        return Ok(());
    }
    let mut rows: Vec<_> = items.into_iter().map(|item| item.columns()).collect();
    let mut table = Table::new();
    table.load_preset(NOTHING);
    match rows.as_mut_slice() {
        [] => return Ok(()),
        [row] => {
            for (label, value) in row.drain(..) {
                table.add_row([format!("{label}:"), value]);
            }
        }
        [first, ..] => {
            table
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(
                    first
                        .iter()
                        .map(|(label, _)| Cell::new(label).add_attribute(Attribute::Bold)),
                );
            table.add_rows(
                rows.into_iter()
                    .map(|row| row.into_iter().map(|(_, value)| value)),
            );
        }
    }
    for column in table.column_iter_mut() {
        column.set_padding((0, 2));
    }
    for line in table.lines() {
        writeln!(out, "{}", line.trim_end())?;
    }
    Ok(())
}
