use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fmt::{self, Display};
use std::fs::{File, OpenOptions};
use std::io::{self, IsTerminal, LineWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use topk_rs::Error as SdkError;

use anyhow::{Context, Result};
use topk_rs::error::DocumentValidationError;
use topk_rs::proto::v1::control::{
    field_index, field_type::DataType, field_type_list::ListValueType, KeywordIndexType,
    VectorDistanceMetric,
};
use topk_rs::proto::v1::data::Document;
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use tracing_indicatif::suspend_tracing_indicatif;

use crate::import::plan::Plan;
use crate::import::state::{Progress, Run};
use crate::import::{Error, Event, Outcome, SEMANTIC_TEXT_LIMIT};
use crate::output::{json_line, Output};
use crate::scope::Scope;

pub struct ImportOutput {
    format: Output,
    status: Span,
    progress: ProgressDisplay,
    outcome: Option<Outcome>,
    /// Where the run lists every skipped row once, one JSON line each, and the rows
    /// already listed: a source row by its number, a destination rejection by its id.
    rejected: Option<(PathBuf, Listed, Option<LineWriter<File>>)>,
}

impl ImportOutput {
    pub fn new(format: Output) -> Self {
        Self {
            format,
            status: Span::none(),
            progress: ProgressDisplay::new(Instant::now()),
            outcome: None,
            rejected: None,
        }
    }

    pub fn message(&self, message: impl Display) -> io::Result<()> {
        suspend_tracing_indicatif(|| writeln!(io::stderr().lock(), "{message}"))
    }

    pub fn confirm(&self, run: &Run, yes: bool) -> Result<bool> {
        let state = run.state();
        self.message(format_args!(
            "# destination: project {}, region {}",
            state.destination.project_id, state.destination.region
        ))?;
        if let Some(Progress::After(resume)) = &state.progress {
            self.message(format_args!("# resuming after {} rows", resume.consumed))?;
        }
        self.message(&state.plan)?;
        crate::prompt::Prompt::confirm(format!("Import into {}?", state.plan.collection), yes)
            .context("pass --yes to import without confirmation")
    }

    pub fn plan(&self, plan: &Plan, scope: &Scope) -> Result<()> {
        self.message(format_args!(
            "# destination: project {}, region {}",
            scope.project_id, scope.region
        ))?;
        match self.format {
            Output::Json => {
                json_line(&mut io::stdout().lock(), plan)?;
                for (name, field) in &plan.target.fields {
                    if matches!(
                        field
                            .schema
                            .data_type
                            .as_ref()
                            .and_then(|ty| ty.data_type.as_ref()),
                        Some(DataType::Text(_))
                    ) && field.schema.as_semantic_index().is_some()
                    {
                        self.message(format_args!("warning: {name} over {SEMANTIC_TEXT_LIMIT} characters will be shortened in the stored document"))?;
                    }
                }
            }
            Output::Text => write!(io::stdout().lock(), "{plan}")?,
        }
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

    pub fn start(&mut self, rejected: PathBuf) -> Result<()> {
        let mut listed = Listed::new();
        match std::fs::read_to_string(&rejected) {
            Ok(lines) => {
                for line in lines.lines() {
                    let line: serde_json::Value = serde_json::from_str(line)?;
                    let row = line["row"].as_u64();
                    listed.insert((
                        line["file"].as_str().map(str::to_string),
                        row,
                        line["id"]
                            .as_str()
                            .filter(|_| row.is_none())
                            .map(str::to_string),
                        line["field"].as_str().map(str::to_string),
                    ));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        self.rejected = Some((rejected, listed, None));
        self.progress = ProgressDisplay::new(Instant::now());
        if self.format == Output::Text && io::stderr().is_terminal() {
            self.status = tracing::info_span!(parent: None, "import_progress", indicatif.pb_show = tracing::field::Empty);
            self.status.pb_start();
        }
        Ok(())
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        if !self.status.is_disabled() {
            self.status.pb_set_message(&self.progress.render(now));
        } else if let Some(line) = self.progress.line(now) {
            let _ = self.message(line);
        }
    }

    pub fn event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Progress { written, bytes } => {
                self.progress.update(written, bytes, Instant::now());
            }
            Event::Skipped { row, part, error } => {
                self.progress.failed += 1;
                match self.progress.failed {
                    1..=3 => self.message(format_args!(
                        "skipped {}{error}",
                        match (row, &part) {
                            (Some(row), Some(part)) => format!("{part} row {row}: "),
                            (Some(row), None) => format!("row {row}: "),
                            (None, _) => String::new(),
                        }
                    ))?,
                    4 => {
                        if let Some((path, ..)) = &self.rejected {
                            self.message(format_args!(
                                "more rows skipped; every skipped row is listed in {}",
                                path.display()
                            ))?;
                        }
                    }
                    _ => {}
                }
                if let Some((path, listed, file)) = &mut self.rejected {
                    let file = match file {
                        Some(file) => file,
                        None => file.insert(LineWriter::new(
                            OpenOptions::new().create(true).append(true).open(&*path)?,
                        )),
                    };
                    let (id, field) = match &error {
                        Error::Doc { id, field, .. } => (id.as_deref(), field.as_deref()),
                        Error::Oversized { id, field, .. } => {
                            (Some(id.as_str()), Some(field.as_str()))
                        }
                        Error::Topk(SdkError::DocumentValidationError(errors)) => match errors
                            .iter()
                            .next()
                        {
                            Some(
                                DocumentValidationError::MissingField { doc_id, field }
                                | DocumentValidationError::ReservedFieldName { doc_id, field }
                                | DocumentValidationError::InvalidFieldName { doc_id, field }
                                | DocumentValidationError::InvalidDataType { doc_id, field, .. }
                                | DocumentValidationError::InvalidVectorDimension {
                                    doc_id,
                                    field,
                                    ..
                                }
                                | DocumentValidationError::InvalidMatrixDimension {
                                    doc_id,
                                    field,
                                    ..
                                }
                                | DocumentValidationError::InvalidMatrix { doc_id, field, .. }
                                | DocumentValidationError::InvalidSparseVector {
                                    doc_id, field, ..
                                }
                                | DocumentValidationError::InvalidStructDepth {
                                    doc_id, field, ..
                                }
                                | DocumentValidationError::InvalidImage { doc_id, field, .. }
                                | DocumentValidationError::TextTooLong { doc_id, field, .. },
                            ) => (Some(doc_id.as_str()), Some(field.as_str())),
                            Some(
                                DocumentValidationError::DocumentTooLarge { doc_id, .. }
                                | DocumentValidationError::DocumentNotFound { doc_id },
                            ) => (Some(doc_id.as_str()), None),
                            _ => (None, None),
                        },
                        _ => (None, None),
                    };
                    if listed.insert((
                        part.clone(),
                        row,
                        id.filter(|_| row.is_none()).map(str::to_string),
                        field.map(str::to_string),
                    )) {
                        json_line(
                            file,
                            &serde_json::json!({"file": part, "row": row, "id": id, "field": field, "error": error.to_string()}),
                        )?;
                    }
                }
            }
            Event::Completed { outcome } => {
                if self.format == Output::Text {
                    self.message(format_args!(
                        "{}{} rows written, {} failed in {:.2?} · {:.0} rows/s · {:.1} MiB/s average",
                        match outcome.resumed_after {
                            0 => String::new(),
                            rows => format!("resumed after {rows} rows: "),
                        },
                        outcome.rows,
                        outcome.failed,
                        outcome.elapsed,
                        outcome.rows as f64 / outcome.elapsed.as_secs_f64().max(f64::EPSILON),
                        outcome.bytes as f64
                            / 1_048_576.0
                            / outcome.elapsed.as_secs_f64().max(f64::EPSILON)
                    ))?;
                }
                self.outcome = Some(outcome);
            }
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.status = Span::none();
        if let (Output::Json, Some(outcome)) = (self.format, &self.outcome) {
            json_line(&mut io::stdout().lock(), outcome)?;
        }
        if let Some((path, listed, _)) = &self.rejected {
            if !listed.is_empty() {
                self.message(format_args!(
                    "{} skipped rows are listed in {}",
                    listed.len(),
                    path.display()
                ))?;
            }
        }
        Ok(())
    }
}

type Listed = HashSet<(Option<String>, Option<u64>, Option<String>, Option<String>)>;

const LINE_EVERY: Duration = Duration::from_secs(10);

struct ProgressDisplay {
    started: Instant,
    printed: Instant,
    last_write: Option<Instant>,
    failed: usize,
    written: (usize, u64),
    samples: VecDeque<(Instant, usize, u64)>,
}

impl ProgressDisplay {
    fn new(now: Instant) -> Self {
        Self {
            started: now,
            printed: now,
            last_write: None,
            failed: 0,
            written: (0, 0),
            samples: VecDeque::from([(now, 0, 0)]),
        }
    }

    fn update(&mut self, rows: usize, bytes: u64, now: Instant) {
        if rows > self.written.0 {
            self.last_write = Some(now);
        }
        self.written = (rows, bytes);
    }

    /// Without a terminal, a plain line every `LINE_EVERY` instead of the redrawn bar.
    fn line(&mut self, now: Instant) -> Option<String> {
        if now.duration_since(self.printed) < LINE_EVERY {
            return None;
        }
        self.printed = now;
        Some(self.render(now))
    }

    fn render(&mut self, now: Instant) -> String {
        let (rows, bytes) = self.written;
        while self.samples.len() > 1
            && now.duration_since(self.samples[1].0) >= Duration::from_secs(5)
        {
            self.samples.pop_front();
        }
        let (sampled, old_rows, old_bytes) = self.samples[0];
        let seconds = now.duration_since(sampled).as_secs_f64().max(f64::EPSILON);
        self.samples.push_back((now, rows, bytes));
        let mut message = format!(
            "{rows} rows · {:.0} rows/s · {:.1} MiB/s · {}s",
            (rows - old_rows) as f64 / seconds,
            (bytes - old_bytes) as f64 / 1_048_576.0 / seconds,
            now.duration_since(self.started).as_secs(),
        );
        if self.failed != 0 {
            message.push_str(&format!(" · {} skipped", self.failed));
        }
        let waiting = now
            .duration_since(self.last_write.unwrap_or(self.started))
            .as_secs();
        if waiting >= 5 {
            if self.last_write.is_some() {
                message.push_str(&format!(" · last write {waiting}s ago"));
            } else {
                message.push_str(&format!(" · waiting for first write {waiting}s"));
            }
        }
        message
    }
}

impl Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (collection, target) = (&self.collection, &self.target);
        writeln!(f, "{collection} ← {}", target.from)?;
        writeln!(f, "  _id ← {}", target.id)?;
        if let Some(partition) = &target.partition {
            writeln!(f, "  partition ← {partition}")?;
        }
        if let Some(filter) = &target.filter {
            writeln!(f, "  filter {filter}")?;
        }
        if let Some(limit) = target.limit {
            writeln!(f, "  limit {limit}")?;
        }
        for (name, field) in &target.fields {
            let ty = field
                .schema
                .data_type
                .as_ref()
                .and_then(|ty| ty.data_type.as_ref());
            let ty = match ty {
                Some(DataType::Text(_)) => "text".to_string(),
                Some(DataType::Integer(_)) => "int".to_string(),
                Some(DataType::Float(_)) => "float".to_string(),
                Some(DataType::Boolean(_)) => "bool".to_string(),
                Some(DataType::Bytes(_)) => "bytes".to_string(),
                Some(DataType::Image(_)) => "image".to_string(),
                Some(DataType::Timestamp(_)) => "timestamp".to_string(),
                Some(DataType::Struct(_)) => "struct".to_string(),
                Some(DataType::List(list)) => match list.value_type() {
                    ListValueType::String => "text_list",
                    ListValueType::Integer => "int_list",
                    ListValueType::Float => "float_list",
                    _ => "list",
                }
                .to_string(),
                Some(DataType::F32Vector(v)) => format!("vector:{}", v.dimension),
                Some(DataType::F16Vector(v)) => format!("f16_vector:{}", v.dimension),
                Some(DataType::F8Vector(v)) => format!("f8_vector:{}", v.dimension),
                Some(DataType::U8Vector(v)) => format!("u8_vector:{}", v.dimension),
                Some(DataType::I8Vector(v)) => format!("i8_vector:{}", v.dimension),
                Some(DataType::BinaryVector(v)) => format!("binary_vector:{}", v.dimension),
                Some(DataType::Matrix(m)) => format!("matrix:{}", m.dimension),
                Some(
                    DataType::F32SparseVector(_)
                    | DataType::F16SparseVector(_)
                    | DataType::F8SparseVector(_)
                    | DataType::U8SparseVector(_)
                    | DataType::I8SparseVector(_),
                ) => "sparse".to_string(),
                _ => "unknown".to_string(),
            };
            let index = match field.schema.index.as_ref().and_then(|i| i.index.as_ref()) {
                Some(field_index::Index::KeywordIndex(i))
                    if i.index_type() == KeywordIndexType::Exact =>
                {
                    "exact".to_string()
                }
                Some(field_index::Index::KeywordIndex(_)) => "keyword".to_string(),
                Some(field_index::Index::SemanticIndex(_)) => "semantic".to_string(),
                Some(field_index::Index::NgramIndex(_)) => "ngram".to_string(),
                Some(field_index::Index::VectorIndex(i)) => match i.metric() {
                    VectorDistanceMetric::Cosine => "cosine",
                    VectorDistanceMetric::Euclidean => "euclidean",
                    VectorDistanceMetric::DotProduct => "dot_product",
                    VectorDistanceMetric::Hamming => "hamming",
                    _ => "unknown",
                }
                .to_string(),
                Some(field_index::Index::MultiVectorIndex(_)) => "maxsim".to_string(),
                None => String::new(),
            };
            writeln!(
                f,
                "  {name}{}: {ty}{}{}",
                field
                    .from
                    .as_ref()
                    .map(|from| format!(" ← {from}"))
                    .unwrap_or_default(),
                if index.is_empty() { "" } else { ", " },
                index
            )?;
            if ty == "text" && index == "semantic" {
                writeln!(f, "  warning: {name} over {SEMANTIC_TEXT_LIMIT} characters will be shortened in the stored document")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tracing_subscriber::prelude::*;

    #[rstest]
    #[case::oversized_document(Error::Oversized { id: "bad".into(), field: "body".into(), size: 2_000_001 })]
    #[case::server_validation(Error::from(topk_rs::Error::DocumentValidationError(topk_rs::error::ValidationErrorBag::new(vec![
        DocumentValidationError::TextTooLong { doc_id: "bad".into(), field: "body".into(), max_length: 64, got_length: 65 },
        DocumentValidationError::MissingField { doc_id: "bad".into(), field: "title".into() },
    ]))))]
    fn rejection_report_keeps_identity(#[case] error: Error) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rejected.jsonl");
        let mut output = ImportOutput::new(Output::Json);
        output.start(path.clone()).unwrap();
        output
            .event(Event::Skipped {
                row: None,
                part: None,
                error,
            })
            .unwrap();
        let report = std::fs::read_to_string(path).unwrap();
        assert_eq!(report.lines().count(), 1);
        let row: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(row["id"], "bad");
        assert_eq!(row["field"], "body");
        assert!(row["error"].as_str().is_some_and(|error| !error.is_empty()));
    }

    #[test]
    fn rejected_rows_listed_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rejected.jsonl");
        let skip = |output: &mut ImportOutput, row| {
            let error = Error::InvalidArgument("bad".into());
            output
                .event(Event::Skipped {
                    row: Some(row),
                    part: None,
                    error,
                })
                .unwrap()
        };
        let mut first = ImportOutput::new(Output::Json);
        first.start(path.clone()).unwrap();
        skip(&mut first, 3);
        let mut resumed = ImportOutput::new(Output::Json);
        resumed.start(path.clone()).unwrap();
        skip(&mut resumed, 3);
        skip(&mut resumed, 4);
        let report = std::fs::read_to_string(path).unwrap();
        assert_eq!(report.lines().count(), 2);
    }

    #[test]
    fn progress_output() {
        let subscriber = tracing_subscriber::registry().with(
            tracing_indicatif::IndicatifLayer::new()
                .with_progress_style(tracing_indicatif::style::ProgressStyle::default_spinner()),
        );
        let _guard = tracing::subscriber::set_default(subscriber);
        let mut output = ImportOutput::new(Output::Text);
        output
            .start(std::env::temp_dir().join("unused.rejected.jsonl"))
            .unwrap();
        assert_eq!(!output.status.is_disabled(), io::stderr().is_terminal());
        output
            .event(Event::Progress {
                written: 100,
                bytes: 1_048_576,
            })
            .unwrap();
        output.progress.started -= Duration::from_secs(2);
        output.progress.samples[0].0 -= Duration::from_secs(2);
        output.tick();
        output
            .event(Event::Completed {
                outcome: Outcome {
                    rows: 100,
                    bytes: 1_048_576,
                    failed: 0,
                    resumed_after: 0,
                    elapsed: Duration::from_secs(2),
                },
            })
            .unwrap();
        assert_eq!(output.progress.written, (100, 1_048_576));
        output.finish().unwrap();
    }

    #[test]
    fn progress_lines_without_terminal() {
        let now = Instant::now();
        let mut progress = ProgressDisplay::new(now);
        assert_eq!(progress.line(now + Duration::from_secs(9)), None);
        progress.update(100, 1_048_576, now + Duration::from_secs(1));
        let line = progress.line(now + Duration::from_secs(10)).unwrap();
        assert!(line.starts_with("100 rows · "), "{line}");
        assert_eq!(progress.line(now + Duration::from_secs(15)), None);
        assert!(progress.line(now + Duration::from_secs(20)).is_some());
    }

    #[rstest]
    #[case::writes_stall(true)]
    #[case::first_write_pending(false)]
    fn progress_during_wait(#[case] written: bool) {
        let now = Instant::now();
        let mut progress = ProgressDisplay::new(now);
        if written {
            progress.update(300, 3_145_728, now + Duration::from_secs(1));
            progress.update(400, 4_194_304, now + Duration::from_secs(2));
            progress.failed = 3;
            assert_eq!(
                progress.render(now + Duration::from_secs(2)),
                "400 rows · 200 rows/s · 2.0 MiB/s · 2s · 3 skipped"
            );
        }
        let mut message = String::new();
        for tick in 9..=40 {
            message = progress.render(now + Duration::from_millis(tick * 250));
        }
        assert!(message.contains("0 rows/s · 0.0 MiB/s"), "{message}");
        assert!(progress.samples.len() <= 22);
        if written {
            assert!(message.ends_with("last write 8s ago"), "{message}");
        } else {
            assert!(
                message.ends_with("waiting for first write 10s"),
                "{message}"
            );
        }
        progress.update(500, 5_242_880, now + Duration::from_secs(11));
        let message = progress.render(now + Duration::from_secs(11));
        assert!(!message.contains("last write"), "{message}");
        assert!(!message.contains("waiting"), "{message}");
        assert!(!message.contains("· 0 rows/s"), "{message}");
    }
}
