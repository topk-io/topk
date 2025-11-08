use std::ffi::CString;
use std::time::Duration;

use async_trait::async_trait;
use pyo3::ffi::c_str;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyListMethods};
use pyo3::{FromPyObject, IntoPyObject, PyResult};
use tokio::time::Instant;

use ::topk_py::data::value::Value as PyValue;
use ::topk_py::data::Document as PyDocument;

use crate::data::Document;

pub mod topk_py;
pub mod topk_rs;
pub mod tpuf_py;

#[async_trait]
pub trait ProviderLike: Send + Sync + 'static {
    /// Setup the state in the provider.
    async fn setup(&self, collection: String) -> anyhow::Result<()>;

    /// Ping the provider, returns duration in milliseconds.
    /// This can be used to estimate if we are running the right region.
    async fn ping(&self, collection: String) -> anyhow::Result<Duration>;

    /// Upsert documents into the provider.
    async fn upsert(&self, collection: String, batch: Vec<Document>) -> anyhow::Result<()>;

    /// Query a document by ID.
    async fn query_by_id(&self, collection: String, id: String)
        -> anyhow::Result<Option<Document>>;

    /// Delete documents by ID.
    #[allow(dead_code)]
    async fn delete_by_id(&self, collection: String, ids: Vec<String>) -> anyhow::Result<()>;

    /// Query documents.
    async fn query(&self, collection: String, query: Query) -> anyhow::Result<Vec<Document>>;

    /// List collections.
    async fn list_collections(&self) -> anyhow::Result<Vec<String>>;

    /// Delete collection.
    async fn delete_collection(&self, collection: String) -> anyhow::Result<()>;

    /// Close the provider.
    async fn close(&self) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct Query {
    /// Vector to query.
    pub(crate) vector: Vec<f32>,
    /// Top K.
    pub(crate) top_k: usize,
    /// Numeric selectivity.
    pub(crate) int_filter: Option<u32>,
    /// Categorical selectivity.
    pub(crate) keyword_filter: Option<String>,
}

#[derive(Clone)]
pub enum Provider {
    /// TopK
    TopkRs(topk_rs::TopkRsProvider),
    TopkPy(topk_py::TopkPyProvider),
    //
    TpufPy(tpuf_py::TpufPyProvider),
}

#[async_trait]
impl ProviderLike for Provider {
    async fn setup(&self, collection: String) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.setup(collection).await,
            Provider::TopkPy(p) => p.setup(collection).await,
            Provider::TpufPy(p) => p.setup(collection).await,
        }
    }

    async fn ping(&self, collection: String) -> Result<Duration, anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.ping(collection).await,
            Provider::TopkPy(p) => p.ping(collection).await,
            Provider::TpufPy(p) => p.ping(collection).await,
        }
    }

    async fn query_by_id(
        &self,
        collection: String,
        id: String,
    ) -> Result<Option<Document>, anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.query_by_id(collection, id).await,
            Provider::TopkPy(p) => p.query_by_id(collection, id).await,
            Provider::TpufPy(p) => p.query_by_id(collection, id).await,
        }
    }

    async fn delete_by_id(
        &self,
        collection: String,
        ids: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.delete_by_id(collection, ids).await,
            Provider::TopkPy(p) => p.delete_by_id(collection, ids).await,
            Provider::TpufPy(p) => p.delete_by_id(collection, ids).await,
        }
    }

    async fn query(
        &self,
        collection: String,
        query: Query,
    ) -> Result<Vec<Document>, anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.query(collection, query).await,
            Provider::TopkPy(p) => p.query(collection, query).await,
            Provider::TpufPy(p) => p.query(collection, query).await,
        }
    }

    async fn upsert(&self, collection: String, batch: Vec<Document>) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.upsert(collection, batch.clone()).await,
            Provider::TopkPy(p) => p.upsert(collection, batch.clone()).await,
            Provider::TpufPy(p) => p.upsert(collection, batch.clone()).await,
        }
    }

    async fn list_collections(&self) -> Result<Vec<String>, anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.list_collections().await,
            Provider::TopkPy(p) => p.list_collections().await,
            Provider::TpufPy(p) => p.list_collections().await,
        }
    }

    async fn delete_collection(&self, collection: String) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.delete_collection(collection).await,
            Provider::TopkPy(p) => p.delete_collection(collection).await,
            Provider::TpufPy(p) => p.delete_collection(collection).await,
        }
    }

    async fn close(&self) -> Result<(), anyhow::Error> {
        match self {
            Provider::TopkRs(p) => p.close().await,
            Provider::TopkPy(p) => p.close().await,
            Provider::TpufPy(p) => p.close().await,
        }
    }
}

#[derive(Clone)]
pub struct PythonProvider {}

impl PythonProvider {
    pub async fn new(code: &'static str) -> anyhow::Result<Self> {
        python_run(move |py| {
            let code = CString::new(code)?;

            py.run(code.as_c_str(), None, None)?;

            Ok(())
        })
        .await?;

        Ok(Self {})
    }
}

#[async_trait]
impl ProviderLike for PythonProvider {
    async fn setup(&self, collection: String) -> anyhow::Result<()> {
        python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("collection", collection.clone())?;

            py.run(c_str!("setup(collection)"), None, Some(&locals))?;

            Ok(())
        })
        .await?;

        Ok(())
    }

    async fn ping(&self, collection: String) -> Result<Duration, anyhow::Error> {
        let start = Instant::now();

        python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("collection", collection.clone())?;
            py.run(c_str!("ping(collection)"), None, Some(&locals))?;

            Ok(())
        })
        .await?;

        Ok(start.elapsed())
    }

    async fn query_by_id(
        &self,
        collection: String,
        id: String,
    ) -> Result<Option<Document>, anyhow::Error> {
        let result = python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("id", id.clone())?;
            locals.set_item("collection", collection.clone())?;

            py.run(
                c_str!("result = query_by_id(collection, id)"),
                None,
                Some(&locals),
            )?;

            let result = locals.get_item("result")?.expect("result is required");
            let result = result.downcast::<PyList>()?;

            Ok(Vec::<PyDocument>::extract_bound(result)?)
        })
        .await?;

        match &result[..] {
            [] => Ok(None),
            [doc] => Ok(Some(Document::new(
                doc.into_iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect(),
            ))),
            _ => anyhow::bail!("expected 1 document, got {}", result.len()),
        }
    }

    async fn delete_by_id(
        &self,
        collection: String,
        ids: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("collection", collection.clone())?;
            locals.set_item("ids", ids.clone())?;

            py.run(c_str!("delete_by_id(collection, ids)"), None, Some(&locals))?;

            Ok(())
        })
        .await?;

        Ok(())
    }

    async fn query(
        &self,
        collection: String,
        query: Query,
    ) -> Result<Vec<Document>, anyhow::Error> {
        let docs = python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("collection", collection.clone())?;
            locals.set_item("vector", query.vector.clone())?;
            locals.set_item("top_k", query.top_k)?;
            locals.set_item("int_filter", query.int_filter)?;
            locals.set_item("keyword_filter", query.keyword_filter.clone())?;

            py.run(
                c_str!("result = query(collection, vector, top_k, int_filter, keyword_filter)"),
                None,
                Some(&locals),
            )?;

            let result = locals.get_item("result")?.expect("result is required");

            let result = result.downcast::<PyList>()?;

            Ok(Vec::<PyDocument>::extract_bound(result)?)
        })
        .await?;

        Ok(docs
            .into_iter()
            .map(|doc| {
                Document::new(
                    doc.into_iter()
                        .map(|(k, v)| (k.clone(), v.clone().into()))
                        .collect(),
                )
            })
            .collect())
    }

    async fn upsert(&self, collection: String, batch: Vec<Document>) -> Result<(), anyhow::Error> {
        python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("collection", collection.clone())?;
            locals.set_item("batch", {
                let list = PyList::empty(py);

                for doc in &batch {
                    let dict = PyDict::new(py);

                    for (key, value) in doc.into_iter() {
                        let topk_py_value = PyValue::from(value);
                        let topk_py_value = topk_py_value.into_pyobject(py)?;
                        dict.set_item(key, topk_py_value)?;
                    }

                    list.append(dict.into_pyobject(py)?)?;
                }

                list
            })?;

            py.run(c_str!("upsert(collection, batch)"), None, Some(&locals))
        })
        .await?;

        Ok(())
    }

    async fn list_collections(&self) -> Result<Vec<String>, anyhow::Error> {
        let collections = python_run(move |py| {
            let locals = PyDict::new(py);

            py.run(
                c_str!("collections = list_collections()"),
                None,
                Some(&locals),
            )?;

            let collections = locals
                .get_item("collections")?
                .expect("collections is required");
            let collections = collections.downcast::<PyList>()?;

            Ok(Vec::<String>::extract_bound(collections)?)
        })
        .await?;

        Ok(collections)
    }

    async fn delete_collection(&self, name: String) -> Result<(), anyhow::Error> {
        python_run(move |py| {
            let locals = PyDict::new(py);
            locals.set_item("name", name.clone())?;

            py.run(c_str!("delete_collection(name)"), None, Some(&locals))?;

            Ok(())
        })
        .await?;

        Ok(())
    }

    async fn close(&self) -> Result<(), anyhow::Error> {
        python_run(|py| py.run(c_str!("close()"), None, None)).await?;
        Ok(())
    }
}

async fn python_run<F, R>(func: F) -> anyhow::Result<R>
where
    F: Fn(pyo3::Python<'_>) -> PyResult<R> + Send + Sync + 'static,
    R: Send + 'static,
{
    let task = tokio::task::spawn_blocking(|| {
        pyo3::Python::with_gil(move |py: pyo3::Python<'_>| Ok(func(py)?))
    });

    task.await.expect("failed to run python code")
}
