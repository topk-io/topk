use chrono::{DateTime, SecondsFormat};
use futures::{stream, Stream, StreamExt};
use topk_rs::proto::v1::data::Document;

use crate::import::sink::build_document;
use crate::import::source::Scan;
use crate::import::{Error, Source, Target, Type, ID};

/// One document per source row, for a read that does not write.
pub fn documents(
    source: &Source,
    target: &Target,
) -> Result<impl Stream<Item = Result<Document, Error>>, Error> {
    let Scan { target, chunks } = source.scan(target, None)?;
    Ok(chunks
        .flat_map(|chunk| {
            stream::iter(match chunk {
                Ok(chunk) => chunk.rows,
                Err(e) => vec![Err(e)],
            })
        })
        .map(move |row| build_document(&target, row?)))
}

const PREVIEW_ROWS: u64 = 5;

/// Characters a previewed document's elidable values share, so a spec with many
/// fields still prints one line each.
const PREVIEW_WIDTH: usize = 200;

/// No field is elided below this, or a timestamp loses its own minutes.
const MIN_VALUE: usize = 24;

/// Documents print on stderr: a preview is what the run is about to write, not
/// the run's result. `json` prints them whole, otherwise each is elided to a
/// line a terminal can hold.
pub async fn preview(source: &Source, target: &Target, json: bool) -> Result<(), Error> {
    let capped = Target {
        limit: Some(target.limit.unwrap_or(u64::MAX).min(PREVIEW_ROWS)),
        ..target.clone()
    };
    let mut rows = Box::pin(documents(source, &capped)?);
    while let Some(row) = rows.next().await {
        let doc = row?;
        let mut pairs = doc
            .fields
            .into_iter()
            .map(|(key, value)| {
                let value = serde_json::Value::try_from(value)?;
                // The schema types the field; the value travels as epoch millis.
                let value = match target.fields.get(&key).map(|field| field.ty) {
                    Some(Type::Timestamp) => instant(&value).unwrap_or(value),
                    _ => value,
                };
                Ok((key, value))
            })
            .collect::<Result<Vec<(String, serde_json::Value)>, topk_rs::Error>>()?;
        pairs.sort_by_key(|(key, _)| (key != ID, key.clone()));
        match json {
            true => eprintln!("{}", serde_json::Value::Object(pairs.into_iter().collect())),
            false => eprintln!("{}", record(&pairs, PREVIEW_WIDTH)),
        }
    }
    Ok(())
}

fn instant(value: &serde_json::Value) -> Option<serde_json::Value> {
    let at = DateTime::from_timestamp_millis(value.as_i64()?)?;
    Some(serde_json::Value::String(
        at.to_rfc3339_opts(SecondsFormat::Secs, true),
    ))
}

/// Every field, each elided to its share of `width`.
fn record(pairs: &[(String, serde_json::Value)], width: usize) -> String {
    let share = width / pairs.len().max(1);
    let fields: Vec<String> = pairs
        .iter()
        .map(|(key, value)| {
            let budget = share.saturating_sub(key.len() + 4).max(MIN_VALUE);
            format!("{key:?}: {}", elide(value, budget))
        })
        .collect();
    format!("{{{}}}", fields.join(", "))
}

/// A value in at most `budget` characters, the rest reported as a count.
fn elide(value: &serde_json::Value, budget: usize) -> String {
    match value {
        serde_json::Value::String(text) => clip(text, budget),
        serde_json::Value::Array(items) => group(
            '[',
            ']',
            items.iter().map(|item| elide(item, budget)),
            items.len(),
            budget,
        ),
        serde_json::Value::Object(entries) => group(
            '{',
            '}',
            entries
                .iter()
                .map(|(key, value)| format!("{key:?}: {}", elide(value, budget))),
            entries.len(),
            budget,
        ),
        other => other.to_string(),
    }
}

/// A quoted string in at most `budget` characters.
pub fn clip(text: &str, budget: usize) -> String {
    match text.chars().count() > budget {
        false => format!("{text:?}"),
        true => {
            let head: String = text.chars().take(budget.max(1)).collect();
            format!("{head:?}…")
        }
    }
}

/// The leading items that fit in `budget`, always at least one, and how many
/// were left out.
fn group(
    open: char,
    close: char,
    items: impl Iterator<Item = String>,
    total: usize,
    budget: usize,
) -> String {
    let mut out = String::from(open);
    let mut shown = 0;
    for item in items {
        if shown > 0 && out.len() + item.len() > budget {
            break;
        }
        if shown > 0 {
            out.push_str(", ");
        }
        out.push_str(&item);
        shown += 1;
    }
    match total - shown {
        0 => format!("{out}{close}"),
        rest => format!("{out}, … {rest} more{close}"),
    }
}
