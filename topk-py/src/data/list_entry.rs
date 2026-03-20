use std::collections::HashMap;

use pyo3::prelude::*;

use crate::data::value::Value;

/// Entry in a dataset.
#[pyclass]
#[derive(Debug, Clone, PartialEq)]
pub struct ListEntry {
    /// Document ID
    #[pyo3(get)]
    pub id: String,
    /// File name
    #[pyo3(get)]
    pub name: String,
    /// File size in bytes
    #[pyo3(get)]
    pub size: u64,
    /// MIME type
    #[pyo3(get)]
    pub mime_type: String,
    /// Metadata fields
    #[pyo3(get)]
    pub metadata: HashMap<String, Value>,
}

#[pymethods]
impl ListEntry {
    pub fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl From<topk_rs::proto::v1::ctx::ListEntry> for ListEntry {
    fn from(entry: topk_rs::proto::v1::ctx::ListEntry) -> Self {
        Self {
            id: entry.id,
            name: entry.name,
            size: entry.size,
            mime_type: entry.mime_type,
            metadata: entry
                .metadata
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}
