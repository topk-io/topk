use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pyo3::types::{PyAnyMethods, PyDict, PyDictMethods, PyList, PyString, PyTuple};
use pyo3::{Py, PyAny, Python};
use serde::Deserialize;
use tokio::time::Instant;
use tracing::info;

use crate::data::{into_python, Document};
use crate::run_python;
use crate::{
    config::LoadConfig,
    providers::{Provider, ProviderLike},
};

const TPUF_COLLECTION: &str = "jobs2";

#[derive(Clone)]
pub struct TpufPyProvider {
    client: Arc<Py<pyo3::types::PyAny>>,
}

#[derive(Deserialize)]
pub struct TpufPySettings {
    pub turbopuffer_api_key: String,
    pub turbopuffer_region: String,
}

impl std::fmt::Debug for TpufPySettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TpufPySettings {{ turbopuffer_api_key: ********, turbopuffer_region: {} }}",
            self.turbopuffer_region
        )
    }
}
/// Creates a new TpufPyProvider.
pub async fn new(_collection: String) -> anyhow::Result<Provider> {
    let settings = TpufPySettings::load_config()?;
    info!(?settings, "Creating TpufPyProvider");

    let client = run_python!(move |py: pyo3::Python<'_>| {
        let config = {
            let dict = pyo3::types::PyDict::new(py);
            dict.set_item("api_key", settings.turbopuffer_api_key.as_str())?;
            dict.set_item("region", settings.turbopuffer_region.as_str())?;
            dict
        };

        let client = py
            .import("turbopuffer")?
            .getattr("Turbopuffer")?
            .call((), Some(&config))?;

        Ok::<Arc<Py<PyAny>>, anyhow::Error>(Arc::new(client.into()))
    })?;

    Ok(Provider::TpufPy(TpufPyProvider { client }))
}

#[async_trait]
impl ProviderLike for TpufPyProvider {
    async fn setup(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn ping(&self) -> anyhow::Result<Duration> {
        let start = Instant::now();
        let client = self.client.clone();

        run_python!(move |py: pyo3::Python<'_>| {
            let args = {
                let args = PyDict::new(py);
                let rank_by = PyTuple::new(
                    py,
                    vec![
                        PyString::new(py, "vector").into_any(),
                        PyString::new(py, "ANN").into_any(),
                        PyList::empty(py).into_any(),
                    ],
                )?;
                args.set_item("rank_by", rank_by)?;
                args.set_item("top_k", 10)?;
                args.into()
            };

            let result = client
                .bind(py)
                .getattr("namespace")?
                .call1(("non-existing-namespace",))?
                .call_method("query", (), Some(&args));

            match result {
                Ok(_) => anyhow::bail!("query should have failed"),
                Err(e) => {
                    let not_found_error = py.import("turbopuffer")?.getattr("NotFoundError")?;
                    let err_value = e.value(py);

                    if err_value.is_instance(&not_found_error)? {
                        let err_message = err_value
                            .getattr("response")?
                            .call_method("json", (), None)?
                            .get_item("error")?
                            .extract::<String>()?;

                        if err_message.contains("namespace")
                            && err_message.contains("was not found")
                        {
                            // Expected error
                        } else {
                            return Err(e.into());
                        }
                    } else {
                        return Err(e.into());
                    }
                }
            }

            Ok(start.elapsed())
        })
    }

    async fn query_by_id(&self, id: String) -> anyhow::Result<Option<Document>> {
        let client = self.client.clone();

        anyhow::bail!("query by id is not supported for TpufPy")
    }

    async fn upsert(&self, batch: Vec<Document>) -> anyhow::Result<()> {
        let client = self.client.clone();

        let docs = batch
            .into_iter()
            .map(|mut doc| {
                Document::new(HashMap::from([
                    ("id".to_string(), doc.remove("id").expect("id is required")),
                    (
                        "text".to_string(),
                        doc.remove("text").expect("text is required"),
                    ),
                    (
                        "vector".to_string(),
                        doc.remove("dense_embedding")
                            .expect("dense_embedding is required"),
                    ),
                    (
                        "numerical_filter".to_string(),
                        doc.remove("numerical_filter")
                            .expect("numerical_filter is required"),
                    ),
                    (
                        "categorical_filter".to_string(),
                        doc.remove("categorical_filter")
                            .expect("categorical_filter is required"),
                    ),
                ]))
            })
            .collect();

        let docs = into_python(docs).await?;

        let _ = run_python!(move |py| -> anyhow::Result<Py<PyAny>> {
            let args = PyDict::new(py);
            args.set_item("upsert_rows", docs)?;
            args.set_item("distance_metric", "cosine_distance")?;
            args.set_item("schema", {
                let schema = PyDict::new(py);
                schema.set_item("text", {
                    let text = PyDict::new(py);
                    text.set_item("type", "string")?;
                    text
                })?;
                schema.set_item("numerical_filter", {
                    let numerical_filter = PyDict::new(py);
                    numerical_filter.set_item("type", "int")?;
                    numerical_filter
                })?;
                schema.set_item("categorical_filter", {
                    let categorical_filter = PyDict::new(py);
                    categorical_filter.set_item("type", "string")?;
                    categorical_filter
                })?;
                schema
            })?;

            let response = client
                .bind(py)
                .getattr("namespace")?
                .call1((TPUF_COLLECTION,))?
                .call_method("write", (), Some(&args))?;

            Ok(response.into())
        });

        Ok(())
    }

    async fn close(&self) -> anyhow::Result<()> {
        // TODO: this tpuf call times out
        // run_python!(move |py| {
        //     self.client.bind(py).call_method0("close")?;
        //     Ok(())
        // })

        Ok(())
    }
}
