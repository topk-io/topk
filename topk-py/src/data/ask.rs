use std::collections::HashMap;

use pyo3::{
    exceptions::PyTypeError,
    prelude::*,
    types::{PyAny, PyDict, PySequence, PyString},
    IntoPyObjectExt,
};
use topk_rs::proto::v1::ctx::ask_result::Message;

use crate::expr::logical::LogicalExpr;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Summarize,
    Reason,
    DeepResearch,
}

impl FromPyObject<'_, '_> for Mode {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        if let Ok(str) = obj.extract::<String>() {
            return match str.as_str() {
                "summarize" => Ok(Mode::Summarize),
                "reason" => Ok(Mode::Reason),
                "deep_research" => Ok(Mode::DeepResearch),
                _ => Err(PyTypeError::new_err(format!(
                    "Invalid mode: {}. Must be one of: summarize, reason, deep_research",
                    str
                ))),
            };
        }

        Err(PyTypeError::new_err(
            "Mode must be one of: summarize, reason, deep_research",
        ))
    }
}

impl From<Mode> for topk_rs::proto::v1::ctx::Mode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Summarize => topk_rs::proto::v1::ctx::Mode::Summarize,
            Mode::Reason => topk_rs::proto::v1::ctx::Mode::Reason,
            Mode::DeepResearch => topk_rs::proto::v1::ctx::Mode::DeepResearch,
        }
    }
}

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

        let filter = match dict.get_item("filter")? {
            None => None,
            Some(fv) if fv.is_none() => None,
            Some(fv) => Some(fv.extract::<LogicalExpr>().map_err(|_| {
                PyTypeError::new_err("Source 'filter' must be a LogicalExpr instance or None")
            })?),
        };

        Ok(Source { dataset, filter })
    }
}

pub struct Sources(Vec<Source>);

impl IntoIterator for Sources {
    type Item = Source;
    type IntoIter = std::vec::IntoIter<Source>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Sources> for Vec<Source> {
    fn from(sources: Sources) -> Self {
        sources.0
    }
}

impl FromPyObject<'_, '_> for Sources {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> PyResult<Self> {
        if obj.is_instance_of::<PyString>() {
            return Err(PyTypeError::new_err(
                "sources must be a list or tuple, not a string; use [\"dataset_name\"] for a single source",
            ));
        }
        let seq = obj.cast::<PySequence>()?;
        let len = seq.len()?;
        let mut sources = Vec::with_capacity(len);
        for i in 0..len {
            let item = seq.get_item(i)?;
            sources.push(Source::from_py_object(&item.as_borrowed())?);
        }
        Ok(Sources(sources))
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
        format!("{:#?}", self)
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
        format!("{:#?}", self)
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
        format!("{:#?}", self)
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
        format!("{:#?}", self)
    }
}

impl From<topk_rs::proto::v1::ctx::Fact> for Fact {
    fn from(f: topk_rs::proto::v1::ctx::Fact) -> Self {
        Fact {
            fact: f.fact,
            source_ids: f.ref_ids,
        }
    }
}

#[pyclass]
#[derive(Clone, PartialEq)]
pub enum Content {
    Text(String),
    Png(Vec<u8>),
    Jpeg(Vec<u8>),
}

impl std::fmt::Debug for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Content::Text(s) => write!(f, "{}", s.split('\n').collect::<Vec<_>>().join("\n")),
            Content::Png(v) => write!(f, "<png {} bytes>", v.len()),
            Content::Jpeg(v) => write!(f, "<jpeg {} bytes>", v.len()),
        }
    }
}

#[pymethods]
impl Content {
    #[getter]
    fn r#type(&self) -> &'static str {
        match self {
            Content::Text(_) => "text",
            Content::Png(_) => "png",
            Content::Jpeg(_) => "jpeg",
        }
    }

    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self {
            Content::Text(s) => s.into_py_any(py),
            Content::Png(v) => v.into_py_any(py),
            Content::Jpeg(v) => v.into_py_any(py),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    content: Content,
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
        use topk_rs::proto::v1::ctx::content::Data;

        let (content, doc_pages) = v
            .content
            .and_then(|c| c.data)
            .map(|data| match data {
                Data::Chunk(chunk) => (Content::Text(chunk.text), chunk.doc_pages),
                Data::Page(page) => {
                    let content = page
                        .image
                        .map(|img| {
                            let data = img.data.to_vec();
                            match img.mime_type.as_str() {
                                s if s.contains("png") => Content::Png(data),
                                _ => Content::Jpeg(data),
                            }
                        })
                        .unwrap_or_else(|| Content::Text(String::new()));
                    (content, vec![page.page_number])
                }
                Data::Image(img) => {
                    let data = img.data.to_vec();
                    let content = match img.mime_type.as_str() {
                        s if s.contains("png") => Content::Png(data),
                        _ => Content::Jpeg(data),
                    };
                    (content, vec![])
                }
            })
            .unwrap_or((Content::Text(String::new()), vec![]));

        SearchResult {
            id: v.doc_id.clone(),
            content,
            doc_id: v.doc_id,
            doc_pages,
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

impl From<topk_rs::proto::v1::ctx::ask_result::Message> for AskResponseMessage {
    fn from(msg: topk_rs::proto::v1::ctx::ask_result::Message) -> Self {
        match msg {
            Message::Answer(fa) => AskResponseMessage::FinalAnswer(FinalAnswer {
                facts: fa.facts.into_iter().map(Fact::from).collect(),
                sources: fa
                    .refs
                    .into_iter()
                    .map(|(k, v)| (k, SearchResult::from(v)))
                    .collect(),
            }),
            Message::Search(sq) => AskResponseMessage::SubQuery(SubQuery {
                objective: sq.objective,
                facts: sq.facts.into_iter().map(Fact::from).collect(),
                sources: sq
                    .refs
                    .into_iter()
                    .map(|(k, v)| (k, SearchResult::from(v)))
                    .collect(),
            }),
            Message::Reason(r) => AskResponseMessage::Reason(Reason { thought: r.thought }),
        }
    }
}
