use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display};
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};
use indexmap::IndexMap;
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
use crate::import::{Event, Outcome, SEMANTIC_TEXT_LIMIT};
use crate::output::{json_line, Output};
use crate::scope::Scope;

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
        let count = pending.len();
        let destination = &run.state().destination;
        self.message(format_args!(
            "# destination: project {}, region {}",
            destination.project_id, destination.region
        ))?;
        for name in pending.keys() {
            if let Some(Progress::After(checkpoint)) = run.state().progress.get(*name) {
                self.message(format_args!(
                    "# {name}: resuming after {} rows",
                    checkpoint.consumed
                ))?;
            }
        }
        self.message(format_args!(
            "{}",
            Plan {
                collections: pending
                    .into_iter()
                    .map(|(name, target)| ((*name).clone(), (*target).clone()))
                    .collect()
            }
        ))?;
        crate::prompt::Prompt::confirm(format!("Import {count} collection(s)?"), yes)
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
                for target in plan.collections.values() {
                    for (name, field) in &target.fields {
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

impl Display for Plan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (collection, target) in &self.collections {
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
        }
        Ok(())
    }
}
