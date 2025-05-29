use pyo3::{exceptions::PyTypeError, prelude::*, types::PyDict};
use std::{sync::Arc, time::Duration};
use topk_rs::ClientConfig;

mod collection;
pub use collection::CollectionClient;

mod collections;
pub use collections::CollectionsClient;

mod runtime;
pub use runtime::Runtime;

#[pyclass]
pub struct Client {
    runtime: Arc<Runtime>,
    client: Arc<topk_rs::Client>,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (api_key, region, host="topk.io".into(), https=true, retry_config=None))]
    pub fn new(
        api_key: String,
        region: String,
        host: String,
        https: bool,
        retry_config: Option<RetryConfig>,
    ) -> Self {
        let runtime = Arc::new(Runtime::new().expect("failed to create runtime"));

        let client = Arc::new(topk_rs::Client::new({
            let mut client = ClientConfig::new(api_key, region)
                .with_https(https)
                .with_host(host);

            if let Some(retry_config) = retry_config {
                client = client.with_retry_config(retry_config.into());
            }

            client
        }));

        Self { runtime, client }
    }

    pub fn collection(&self, collection: String) -> PyResult<CollectionClient> {
        Ok(CollectionClient::new(
            self.runtime.clone(),
            self.client.clone(),
            collection,
        ))
    }

    pub fn collections(&self) -> PyResult<CollectionsClient> {
        Ok(CollectionsClient::new(
            self.runtime.clone(),
            self.client.clone(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: Option<usize>,

    /// Total timeout for the retry chain (milliseconds)
    pub timeout: Option<u64>,

    /// Backoff configuration
    pub backoff: Option<BackoffConfig>,
}

impl<'py> FromPyObject<'py> for RetryConfig {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        match obj.downcast::<PyDict>() {
            Ok(dict) => {
                let max_retries = dict
                    .get_item("max_retries")?
                    .map(|v| v.extract::<usize>())
                    .transpose()?;

                let timeout = dict
                    .get_item("timeout")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?;

                let backoff = dict
                    .get_item("backoff")?
                    .map(|v| v.extract::<BackoffConfig>())
                    .transpose()?;

                Ok(RetryConfig {
                    max_retries,
                    timeout,
                    backoff,
                })
            }
            _ => Err(PyTypeError::new_err("`RetryConfig` must be a dict")),
        }
    }
}

impl Into<topk_rs::retry::RetryConfig> for RetryConfig {
    fn into(self) -> topk_rs::retry::RetryConfig {
        topk_rs::retry::RetryConfig {
            max_retries: self
                .max_retries
                .unwrap_or(topk_rs::defaults::RETRY_MAX_RETRIES) as usize,
            timeout: Duration::from_millis(
                self.timeout.unwrap_or(topk_rs::defaults::RETRY_TIMEOUT),
            ),
            backoff: self.backoff.map(|b| b.into()).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Base for the backoff
    pub base: Option<u32>,

    /// Initial backoff (milliseconds)
    pub init_backoff: Option<u64>,

    /// Maximum backoff (milliseconds)
    pub max_backoff: Option<u64>,
}

impl<'py> FromPyObject<'py> for BackoffConfig {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        match obj.downcast::<PyDict>() {
            Ok(dict) => {
                let base = dict
                    .get_item("base")?
                    .map(|v| v.extract::<u32>())
                    .transpose()?;

                let init_backoff = dict
                    .get_item("init_backoff")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?;

                let max_backoff = dict
                    .get_item("max_backoff")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?;

                Ok(BackoffConfig {
                    base,
                    init_backoff,
                    max_backoff,
                })
            }
            _ => Err(PyTypeError::new_err("`BackoffConfig` must be a dict")),
        }
    }
}
impl Into<topk_rs::retry::BackoffConfig> for BackoffConfig {
    fn into(self) -> topk_rs::retry::BackoffConfig {
        topk_rs::retry::BackoffConfig {
            base: self.base.unwrap_or(topk_rs::defaults::RETRY_BACKOFF_BASE),
            init_backoff: Duration::from_millis(
                self.init_backoff
                    .unwrap_or(topk_rs::defaults::RETRY_BACKOFF_INIT),
            ),
            max_backoff: Duration::from_millis(
                self.max_backoff
                    .unwrap_or(topk_rs::defaults::RETRY_BACKOFF_MAX),
            ),
        }
    }
}
