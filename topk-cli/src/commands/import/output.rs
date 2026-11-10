use std::collections::BTreeMap;
use std::process::ExitCode;

use anyhow::{Context, Result};
use futures::StreamExt;
use indexmap::IndexMap;

use crate::import::{self, LoadOutcome, Mark, Run, Spec, Target, ID, ID_PLACEHOLDER};
use crate::output::Output;

const PREVIEW_ROWS: u64 = 5;
const PREVIEW_ELEMENTS: usize = 8;
const PREVIEW_CHARS: usize = 120;

pub async fn preview(source: &import::Source, spec: &Spec, id_hint: &str) -> Result<()> {
    print!("{}", toml::to_string_pretty(spec)?);
    for (name, target) in &spec.collections {
        if target.id.as_deref() == Some(ID_PLACEHOLDER) {
            eprintln!("# {name}: {id_hint} to preview rows");
            continue;
        }
        let target = Target {
            limit: Some(target.limit.unwrap_or(u64::MAX).min(PREVIEW_ROWS)),
            ..target.clone()
        };
        let mut rows = import::documents(source, &target)?;
        let mut shown = 0;
        while let Some(row) = rows.next().await {
            let (partition, doc) = row?;
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
            let note = partition.map_or_else(String::new, |p| format!(" # partition {p}"));
            eprintln!("{}{note}", elide(&doc));
        }
        if shown == PREVIEW_ROWS {
            eprintln!("# … showing the first {PREVIEW_ROWS} rows");
        }
    }
    Ok(())
}

pub fn confirm(run: &Run, yes: bool) -> Result<bool> {
    let spec: IndexMap<_, _> = run.pending().collect();
    let done = run.spec.collections.len() - spec.len();
    // Before the run: a killed run prints nothing after.
    eprintln!(
        "# run {}{}",
        run.id,
        match done {
            0 => String::new(),
            n => format!(", resuming: {n} collection(s) done"),
        }
    );
    for name in spec.keys() {
        if let Some(Mark::After(checkpoint)) = run.cursors.get(*name) {
            eprintln!(
                "# resuming after {}",
                elide(&serde_json::Value::String(checkpoint.cursor.to_string()))
            );
        }
    }
    eprint!("{}", toml::to_string_pretty(&spec)?);
    let collections = spec.len();
    let s = if collections == 1 { "" } else { "s" };
    crate::util::confirm(
        format!(
            "Import {collections} collection{s} into {}?",
            run.destination.region
        ),
        yes,
    )
    .context("pass --yes to import without confirmation")
}

pub fn completed(run: &Run, output: Output) -> Result<ExitCode> {
    eprintln!(
        "run {}: all {} collection(s) already imported",
        run.id,
        run.spec.collections.len()
    );
    report(&BTreeMap::new(), output)
}

pub fn report(outcomes: &BTreeMap<String, LoadOutcome>, output: Output) -> Result<ExitCode> {
    if output == Output::Json {
        println!("{}", serde_json::to_string(outcomes)?);
    } else {
        for (name, outcome) in outcomes {
            let secs = outcome.elapsed.as_secs();
            let took = match (secs / 3600, secs % 3600 / 60, secs % 60) {
                (0, 0, s) => format!("{s}s"),
                (0, m, s) => format!("{m}m{s:02}s"),
                (h, m, s) => format!("{h}h{m:02}m{s:02}s"),
            };
            match outcome.failed {
                0 => eprintln!("{name}: {} rows written in {took}", outcome.rows),
                failed => eprintln!(
                    "{name}: {} rows written, {failed} failed in {took}",
                    outcome.rows
                ),
            }
        }
    }
    Ok(ExitCode::from(
        outcomes.values().any(|o| o.failed != 0) as u8
    ))
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
            let pair =
                |(key, value): (&String, &serde_json::Value)| format!("{key:?}: {}", elide(value));
            if entries.len() > PREVIEW_ELEMENTS {
                let head: Vec<String> = entries.iter().take(2).map(pair).collect();
                let tail: Vec<String> = entries.iter().skip(entries.len() - 2).map(pair).collect();
                return format!(
                    "{{{}, … {} pairs, {}}}",
                    head.join(", "),
                    entries.len(),
                    tail.join(", ")
                );
            }
            let pairs: Vec<String> = entries.iter().map(pair).collect();
            format!("{{{}}}", pairs.join(", "))
        }
        other => other.to_string(),
    }
}
