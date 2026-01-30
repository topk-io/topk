use std::collections::HashMap;

use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyDict, PyList},
};

use crate::expr::logical::LogicalExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum Effort {
    Unspecified,
    Medium,
    Low,
    High,
}

impl FromPyObject<'_, '_> for Effort {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        if let Ok(str) = obj.extract::<String>() {
            return match str.as_str() {
                "medium" => Ok(Effort::Medium),
                "low" => Ok(Effort::Low),
                "high" => Ok(Effort::High),
                _ => Err(PyTypeError::new_err(format!(
                    "Invalid effort value: {}. Must be one of: medium, low, high",
                    str
                ))),
            };
        }

        Err(PyTypeError::new_err(
            "Effort must be either (medium, low, high)",
        ))
    }
}

impl From<Effort> for topk_rs::proto::v1::ctx::Effort {
    fn from(effort: Effort) -> Self {
        match effort {
            Effort::Unspecified => topk_rs::proto::v1::ctx::Effort::Unspecified,
            Effort::Medium => topk_rs::proto::v1::ctx::Effort::Medium,
            Effort::Low => topk_rs::proto::v1::ctx::Effort::Low,
            Effort::High => topk_rs::proto::v1::ctx::Effort::High,
        }
    }
}

impl From<topk_rs::proto::v1::ctx::Effort> for Effort {
    fn from(effort: topk_rs::proto::v1::ctx::Effort) -> Self {
        match effort {
            topk_rs::proto::v1::ctx::Effort::Unspecified => Effort::Unspecified,
            topk_rs::proto::v1::ctx::Effort::Medium => Effort::Medium,
            topk_rs::proto::v1::ctx::Effort::Low => Effort::Low,
            topk_rs::proto::v1::ctx::Effort::High => Effort::High,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub dataset: String,
    pub filter: Option<LogicalExpr>,
}

impl Source {
    fn from_py_object(obj: &Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        // If it's a string, treat it as just a dataset name (no filter)
        if let Ok(dataset) = obj.extract::<String>() {
            return Ok(Source {
                dataset,
                filter: None,
            });
        }

        // Otherwise, try to extract as a dict
        let dict = obj.cast_exact::<PyDict>().map_err(|_| {
          PyTypeError::new_err("Source must be a string (dataset name) or a dict with 'dataset' and optional 'filter' keys")
      })?;

        let dataset = dict
            .get_item("dataset")?
            .ok_or_else(|| PyTypeError::new_err("Source dict must have 'dataset' key"))?
            .extract::<String>()
            .map_err(|_| PyTypeError::new_err("Source 'dataset' must be a string"))?;

        let filter = if let Some(filter_val) = dict.get_item("filter")? {
            if filter_val.is_none() {
                None
            } else {
                Some(filter_val.extract::<LogicalExpr>().map_err(|_| {
                    PyTypeError::new_err("Source 'filter' must be a LogicalExpr instance or None")
                })?)
            }
        } else {
            None
        };

        Ok(Source { dataset, filter })
    }
}

pub struct Sources(Vec<Source>);

impl From<Sources> for Vec<Source> {
    fn from(sources: Sources) -> Self {
        sources.0
    }
}

impl FromPyObject<'_, '_> for Sources {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        let list = obj.cast::<PyList>()?;
        Ok(Sources(
            list.iter()
                .map(|item| Source::from_py_object(&item.as_borrowed()))
                .collect::<PyResult<Vec<Source>>>()?,
        ))
    }
}

impl From<Source> for topk_rs::proto::v1::ctx::Source {
    fn from(source: Source) -> Self {
        topk_rs::proto::v1::ctx::Source {
            dataset: source.dataset,
            filter: source.filter.map(|f| f.into()),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct FinalAnswer {
    #[pyo3(get)]
    facts: Vec<Fact>,
    #[pyo3(get)]
    sources: HashMap<String, SearchResult>,
}

#[pymethods]
impl FinalAnswer {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct SubQuery {
    #[pyo3(get)]
    objective: String,
    #[pyo3(get)]
    facts: Vec<Fact>,
    #[pyo3(get)]
    sources: HashMap<String, SearchResult>,
}

#[pymethods]
impl SubQuery {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Reason {
    #[pyo3(get)]
    thought: String,
}

#[pymethods]
impl Reason {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    #[pyo3(get)]
    fact: String,
    #[pyo3(get)]
    source_ids: Vec<String>,
}

#[pymethods]
impl Fact {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<topk_rs::proto::v1::ctx::Fact> for Fact {
    fn from(f: topk_rs::proto::v1::ctx::Fact) -> Self {
        Fact {
            fact: f.fact,
            source_ids: f.source_ids,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    content: String,
    #[pyo3(get)]
    doc_id: String,
    #[pyo3(get)]
    doc_pages: Vec<u32>,
}

#[pymethods]
impl SearchResult {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<topk_rs::proto::v1::ctx::SearchResult> for SearchResult {
    fn from(v: topk_rs::proto::v1::ctx::SearchResult) -> Self {
        SearchResult {
            id: v.id,
            content: v.content,
            doc_id: v.doc_id,
            doc_pages: v.doc_pages,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AskResponseMessage {
    FinalAnswer(FinalAnswer),
    SubQuery(SubQuery),
    Reason(Reason),
}

impl<'py> IntoPyObject<'py> for AskResponseMessage {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> PyResult<Self::Output> {
        match self {
            AskResponseMessage::FinalAnswer(final_answer) => {
                Ok(Py::new(py, final_answer)?.into_bound(py).into_any())
            }
            AskResponseMessage::SubQuery(sub_query) => {
                Ok(Py::new(py, sub_query)?.into_bound(py).into_any())
            }
            AskResponseMessage::Reason(reason) => {
                Ok(Py::new(py, reason)?.into_bound(py).into_any())
            }
        }
    }
}

impl From<topk_rs::proto::v1::ctx::AskResponseMessage> for AskResponseMessage {
    fn from(msg: topk_rs::proto::v1::ctx::AskResponseMessage) -> Self {
        use topk_rs::proto::v1::ctx::ask_response_message::Message;

        match msg.message {
            Some(Message::FinalAnswer(fa)) => AskResponseMessage::FinalAnswer(FinalAnswer {
                facts: fa.facts.into_iter().map(Fact::from).collect(),
                sources: fa
                    .sources
                    .into_iter()
                    .map(|(k, v)| (k, SearchResult::from(v)))
                    .collect(),
            }),
            Some(Message::SubQuery(sq)) => AskResponseMessage::SubQuery(SubQuery {
                objective: sq.objective,
                facts: sq.facts.into_iter().map(Fact::from).collect(),
                sources: sq
                    .sources
                    .into_iter()
                    .map(|(k, v)| (k, SearchResult::from(v)))
                    .collect(),
            }),
            Some(Message::Reason(r)) => AskResponseMessage::Reason(Reason { thought: r.thought }),
            None => AskResponseMessage::Reason(Reason {
                thought: "Unknown message type".to_string(),
            }),
        }
    }
}
