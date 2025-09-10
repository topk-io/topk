use crate::client::Document;
use crate::data::value::Value;
use crate::error::RustError;
use crate::query::{ConsistencyLevel, Query};
use pyo3::prelude::*;
use pyo3_async_runtimes::tokio::future_into_py;
use std::{collections::HashMap, sync::Arc};

#[pyclass]
pub struct AsyncCollectionClient {
    client: Arc<topk_rs::Client>,
    collection: Arc<String>,
}

impl AsyncCollectionClient {
    pub fn new(client: Arc<topk_rs::Client>, collection: Arc<String>) -> Self {
        Self { client, collection }
    }
}

#[pymethods]
impl AsyncCollectionClient {
    #[pyo3(signature = (ids, fields=None, lsn=None, consistency=None))]
    pub fn get(
        &self,
        py: Python<'_>,
        ids: Vec<String>,
        fields: Option<Vec<String>>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<PyObject> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        future_into_py(py, async move {
            let docs = client
                .collection(collection.as_str())
                .get(ids, fields, lsn, consistency.map(|c| c.into()))
                .await
                .map_err(RustError)?;

            let docs: HashMap<String, Document> = docs
                .into_iter()
                .map(|(id, doc)| (id, Document::from(doc)))
                .collect();

            Ok(docs)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (lsn=None, consistency=None))]
    pub fn count(
        &self,
        py: Python<'_>,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<PyObject> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        future_into_py(py, async move {
            let count = client
                .collection(collection.as_str())
                .count(lsn, consistency.map(|c| c.into()))
                .await
                .map_err(RustError)?;

            Ok(count)
        })
        .map(|result| result.into())
    }

    #[pyo3(signature = (query, lsn=None, consistency=None))]
    pub fn query(
        &self,
        py: Python<'_>,
        query: Query,
        lsn: Option<String>,
        consistency: Option<ConsistencyLevel>,
    ) -> PyResult<PyObject> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        future_into_py(py, async move {
            let docs = client
                .collection(collection.as_str())
                .query(query.into(), lsn, consistency.map(|c| c.into()))
                .await
                .map_err(RustError)?;

            let docs: Vec<Document> = docs.into_iter().map(|d| d.into()).collect();

            Ok(docs)
        })
        .map(|result| result.into())
    }

    pub fn upsert(
        &self,
        py: Python<'_>,
        documents: Vec<HashMap<String, Value>>,
    ) -> PyResult<PyObject> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        future_into_py(py, async move {
            let documents = documents
                .into_iter()
                .map(|d| topk_rs::proto::v1::data::Document {
                    fields: d.into_iter().map(|(k, v)| (k, v.into())).collect(),
                })
                .collect();

            let lsn = client
                .collection(collection.as_str())
                .upsert(documents)
                .await
                .map_err(RustError)?;

            Ok(lsn)
        })
        .map(|result| result.into())
    }

    pub fn delete(&self, py: Python<'_>, ids: Vec<String>) -> PyResult<PyObject> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        future_into_py(py, async move {
            let lsn = client
                .collection(collection.as_str())
                .delete(ids)
                .await
                .map_err(RustError)?;

            Ok(lsn)
        })
        .map(|result| result.into())
    }
}
