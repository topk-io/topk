use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use topk_rs::proto::v1::data::Document;
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use tracing_indicatif::suspend_tracing_indicatif;

use crate::import::spec::Spec;
use crate::import::state::{Progress, Run};
use crate::import::{Event, Outcome};
use crate::output::{json_line, Output};

pub struct ImportOutput {
    format: Output,
    status: Span,
    skipped: HashMap<String, usize>,
    outcomes: BTreeMap<String, Outcome>,
}

impl ImportOutput {
    pub fn new(format: Output) -> Self {
        Self {
            format,
            status: Span::none(),
            skipped: HashMap::new(),
            outcomes: BTreeMap::new(),
        }
    }

    pub fn message(&self, message: impl Display) -> io::Result<()> {
        suspend_tracing_indicatif(|| writeln!(io::stderr().lock(), "{message}"))
    }

    pub fn confirm(&self, run: &Run, yes: bool) -> Result<bool> {
        let pending: IndexMap<_, _> = run.state().pending().collect();
        for name in pending.keys() {
            if let Some(Progress::After(checkpoint)) = run.state().progress.get(*name) {
                self.message(format_args!(
                    "# {name}: resuming after {} rows",
                    checkpoint.consumed
                ))?;
            }
        }
        self.message(toml::to_string_pretty(&pending)?)?;
        crate::prompt::Prompt::confirm(
            format!(
                "Import {} collection(s) into {}?",
                pending.len(),
                run.state().destination.region
            ),
            yes,
        )
        .context("pass --yes to import without confirmation")
    }

    pub fn spec(&self, spec: &Spec) -> Result<()> {
        write!(io::stdout().lock(), "{}", toml::to_string_pretty(spec)?)?;
        Ok(())
    }

    pub fn sample_header(&self, collection: &str) -> io::Result<()> {
        self.message(format_args!("# → {collection} (sample, up to 5 rows)"))
    }

    pub fn sample(&self, partition: Option<&str>, document: Document) -> Result<()> {
        let fields = document
            .fields
            .into_iter()
            .map(|(name, value)| Ok((name, serde_json::Value::try_from(value)?)))
            .collect::<Result<BTreeMap<_, _>, topk_rs::Error>>()?;
        let mut text = serde_json::to_string(&fields)?;
        if let Some((end, _)) = text.char_indices().nth(240) {
            text.truncate(end);
            text.push('…');
        }
        if let Some(partition) = partition {
            text.push_str(&format!(" # partition {partition}"));
        }
        self.message(text)?;
        Ok(())
    }

    pub fn event(&mut self, event: Event) -> Result<()> {
        if self.status.is_disabled() && self.format == Output::Text && io::stderr().is_terminal() {
            self.status = tracing::info_span!(parent: None, "import_progress", indicatif.pb_show = tracing::field::Empty);
            self.status.pb_start();
        }
        match event {
            Event::Progress {
                collection,
                written,
            } => {
                self.status
                    .pb_set_message(&format!("{collection}: {written} rows written"));
            }
            Event::Skipped { collection, error } => {
                let count = self.skipped.entry(collection.clone()).or_default();
                *count += 1;
                match *count {
                    1..=3 => self.message(format_args!("{collection}: skipped {error}"))?,
                    4 => self.message(format_args!(
                        "{collection}: skipping more failing rows; the summary counts them"
                    ))?,
                    _ => {}
                }
            }
            Event::Completed {
                collection,
                outcome,
            } => {
                if self.format == Output::Text {
                    self.message(format_args!(
                        "{collection}: {} rows written, {} failed in {:.2?}",
                        outcome.rows, outcome.failed, outcome.elapsed
                    ))?;
                }
                self.outcomes.insert(collection, outcome);
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<ExitCode> {
        self.status = Span::none();
        if self.format == Output::Json {
            json_line(&mut io::stdout().lock(), &self.outcomes)?;
        }
        Ok(ExitCode::from(
            self.outcomes.values().any(|outcome| outcome.failed != 0) as u8,
        ))
    }
}
