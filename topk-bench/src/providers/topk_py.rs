use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pyo3::types::{PyAnyMethods, PyDict};
use pyo3::FromPyObject;
use pyo3::{Py, PyAny, Python};
use serde::Deserialize;
use tokio::time::Instant;
use tracing::info;

use topk_py::data::Document as PyDocument;
use topk_rs::doc;

use crate::data::into_python;
use crate::run_python;
use crate::{
    config::LoadConfig,
    data::Document,
    providers::{Provider, ProviderLike},
};

#[derive(Clone)]
pub struct TopkPyProvider {
    /// Topk-py client (Python object)
    client: Arc<Py<PyAny>>,

    /// Collection name
    collection: String,
}

#[derive(Deserialize)]
pub struct TopkPySettings {
    /// Topk API key
    pub topk_api_key: String,
    /// Topk region
    pub topk_region: String,
    /// Topk host
    pub topk_host: Option<String>,
    /// Topk HTTPS
    pub topk_https: Option<bool>,
}

impl std::fmt::Debug for TopkPySettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TopkPySettings {{ topk_api_key: ********, topk_region: {}, topk_host: {:?}, topk_https: {:?} }}",
            self.topk_region, self.topk_host, self.topk_https
        )
    }
}

/// Creates a new TopkRsProvider.
pub async fn new(collection: String) -> anyhow::Result<Provider> {
    let settings = TopkPySettings::load_config()?;
    info!(?settings, "Creating TopkPyProvider");

    let client = run_python!(move |py: pyo3::Python<'_>| -> anyhow::Result<Py<PyAny>> {
        // Create client instance with settings
        let config = {
            let dict = PyDict::new(py);
            dict.set_item("api_key", &settings.topk_api_key)?;
            dict.set_item("region", &settings.topk_region)?;
            dict.set_item("host", &settings.topk_host.unwrap_or("topk.io".into()))?;
            dict.set_item("https", settings.topk_https.unwrap_or(true))?;
            dict
        };

        let client = py
            .import("topk_sdk")?
            .getattr("Client")?
            .call((), Some(&config))?;

        Ok(client.into())
    })?;

    Ok(Provider::TopkPy(TopkPyProvider {
        client: Arc::new(client),
        collection,
    }))
}

#[async_trait]
impl ProviderLike for TopkPyProvider {
    async fn setup(&self) -> anyhow::Result<()> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        info!("Setting up collection");
        run_python!(|py| -> anyhow::Result<()> {
            client
                .bind(py)
                .call_method("collections", (), None)?
                .call_method("get", (collection,), None)?;

            Ok(())
        })?;

        Ok(())
    }

    async fn ping(&self) -> anyhow::Result<Duration> {
        let start = Instant::now();
        let client = self.client.clone();

        run_python!(move |py: pyo3::Python<'_>| -> anyhow::Result<()> {
            let collection_not_found_error = py
                .import("topk_sdk.error")?
                .getattr("CollectionNotFoundError")?;

            let result = client
                .bind(py)
                .call_method("collections", (), None)?
                .call_method("get", ("non-existing-collection",), None);

            match result {
                Ok(_) => anyhow::bail!("get should have failed"),
                Err(e) => {
                    if e.value(py).is_instance(&collection_not_found_error)? {
                        Ok(())
                    } else {
                        Err(e.into())
                    }
                }
            }
        })?;

        Ok(start.elapsed())
    }

    async fn query_by_id(&self, id: String) -> anyhow::Result<Option<Document>> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        let py_documents = run_python!(
            move |py: pyo3::Python<'_>| -> anyhow::Result<Vec<PyDocument>> {
                let filter = py.import("topk_sdk.query")?.getattr("filter")?;

                // `field("_id").eq(id)`
                let expr = py
                    .import("topk_sdk.query")?
                    .getattr("field")?
                    .call1(("_id".to_string(),))?
                    .call_method("eq", (id,), None)?;

                // `filter(expr)`
                let query = filter.call1((expr,))?.call_method("limit", (1,), None)?;

                let result = client
                    .bind(py)
                    .call_method("collection", (collection,), None)?
                    .call_method("query", (query,), None)?;

                Ok(Vec::<PyDocument>::extract_bound(&result)?)
            }
        )?;

        match &py_documents[..] {
            [] => Ok(None),
            [doc] => Ok(Some(Document::new(
                doc.into_iter()
                    .map(|(k, v)| (k.clone(), v.clone().into()))
                    .collect(),
            ))),
            _ => anyhow::bail!("expected 1 document, got {}", py_documents.len()),
        }
    }

    async fn upsert(&self, batch: Vec<Document>) -> anyhow::Result<()> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        let batch = batch
            .into_iter()
            .map(|mut doc| {
                if let Some(id_val) = doc.remove("id") {
                    doc.insert("_id", id_val);
                }
                doc.into()
            })
            .collect();
        let documents = into_python(batch).await?;

        let _ = run_python!(move |py| -> anyhow::Result<String> {
            let lsn = client
                .bind(py)
                .getattr("collection")?
                .call1((collection,))?
                .getattr("upsert")?
                .call1((documents,))?;

            Ok(lsn.extract()?)
        });

        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        Ok(())
    }
}
