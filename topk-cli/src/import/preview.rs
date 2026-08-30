use futures::StreamExt;

use crate::import::{self, Error, Target, ID};

const PREVIEW_ROWS: usize = 5;
const PREVIEW_ELEMENTS: usize = 8;
const PREVIEW_CHARS: usize = 120;

pub async fn preview(name: &str, source: &import::Source, target: &Target) -> Result<(), Error> {
    // Cap at the source: a dropped ES stream leaks its point-in-time.
    let cap = PREVIEW_ROWS as u64;
    let target = Target {
        limit: Some(target.limit.map_or(cap, |limit| limit.min(cap))),
        ..target.clone()
    };
    let mut rows = import::documents(source.scan(name, &target, None)?).await?;
    let mut shown = 0;
    while let Some(row) = rows.next().await {
        let doc = row?;
        if shown == 0 {
            eprintln!("# → {name}");
        }
        shown += 1;

        let mut pairs = doc
            .fields
            .into_iter()
            .map(|(key, value)| Ok((key, serde_json::Value::try_from(value)?)))
            .collect::<Result<Vec<(String, serde_json::Value)>, topk_rs::Error>>()?;
        pairs.sort_by_key(|(k, _)| (k != ID, k.clone()));
        let doc = serde_json::Value::Object(pairs.into_iter().collect());
        // stderr, so `--dry-run > spec.toml` captures the spec alone.
        eprintln!("{}", elide(&doc));
    }
    if shown as u64 == cap {
        eprintln!("# … showing the first {PREVIEW_ROWS} rows");
    }
    Ok(())
}

pub fn elide(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Array(items) if items.len() > PREVIEW_ELEMENTS => {
            let head: Vec<String> = items.iter().take(2).map(elide).collect();
            let tail: Vec<String> = items[items.len() - 2..].iter().map(elide).collect();
            format!(
                "[{}, … {} values, {}]",
                head.join(", "),
                items.len(),
                tail.join(", ")
            )
        }
        serde_json::Value::String(text) if text.chars().count() > PREVIEW_CHARS => {
            let head: String = text.chars().take(PREVIEW_CHARS).collect();
            format!("{head:?}…")
        }
        serde_json::Value::Object(entries) => {
            let pairs: Vec<String> = entries
                .iter()
                .map(|(key, value)| format!("{key:?}: {}", elide(value)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        other => other.to_string(),
    }
}
