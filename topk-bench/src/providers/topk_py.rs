use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use topk_rs::proto::v1::data::Document;
use tracing::info;

use async_trait::async_trait;
use pyo3::types::{PyAnyMethods, PyDict};
use pyo3::{Py, PyAny, Python};
use serde::Deserialize;
use topk_rs::doc;

use crate::data::into_python;
use crate::run_python;
use crate::{
    config::LoadConfig,
    providers::{Provider, ProviderLike},
};

#[derive(Clone)]
pub struct TopkPyProvider {
    /// Topk-py client (Python object)
    client: Arc<Py<pyo3::types::PyAny>>,

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

    let client = run_python!(move |py: pyo3::Python<'_>| {
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

        Ok::<Arc<Py<PyAny>>, anyhow::Error>(Arc::new(client.into()))
    })?;

    Ok(Provider::TopkPy(TopkPyProvider { client, collection }))
}

#[async_trait]
impl ProviderLike for TopkPyProvider {
    async fn setup(&self) -> anyhow::Result<()> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        info!("Setting up collection");
        let collection = run_python!(|py| {
            let c = client
                .bind(py)
                .getattr("collections")?
                .call0()?
                .getattr("get")?
                .call1((collection,))?;

            Ok::<Py<pyo3::types::PyAny>, anyhow::Error>(c.into())
        })?;
        info!("Collection set up: {:?}", collection);

        Ok(())
    }

    async fn ping(&self) -> anyhow::Result<Duration> {
        let start = Instant::now();
        let client = self.client.clone();

        run_python!(move |py: pyo3::Python<'_>| -> anyhow::Result<Duration> {
            let select = py.import("topk_sdk.query")?.getattr("select")?;

            let random_collection = format!(
                "random-collection-{}",
                rand::thread_rng().gen_range(1..1000000)
            );

            let result = client
                .bind(py)
                .call_method("collection", (random_collection,), None)?
                .call_method("query", (select.call0()?,), None);

            match result {
                Ok(_) => anyhow::bail!("query should have failed"),
                Err(e) => {
                    let collection_not_found_error = py
                        .import("topk_sdk.error")?
                        .getattr("CollectionNotFoundError")?;

                    if e.value(py).is_instance(&collection_not_found_error)? {
                        // Expected error
                    } else {
                        return Err(e.into());
                    }
                }
            }

            Ok(start.elapsed())
        })
    }

    async fn upsert(&self, batch: Vec<Document>) -> anyhow::Result<()> {
        let client = self.client.clone();
        let collection = self.collection.clone();

        let batch = batch
            .into_iter()
            .map(|mut doc| {
                if let Some(id_val) = doc.fields.remove("id") {
                    doc.fields.insert("_id".to_string(), id_val);
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
