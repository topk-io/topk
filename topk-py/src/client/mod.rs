use pyo3::{exceptions::PyTypeError, prelude::*, types::PyDict, IntoPyObject};
use std::{collections::HashMap, sync::Arc, time::Duration};
use topk_rs::ClientConfig;

use crate::data::value::NativeValue;

pub mod r#async;
pub mod sync;

pub fn topk_client(
    api_key: String,
    region: String,
    host: String,
    https: bool,
    retry_config: Option<RetryConfig>,
) -> Arc<topk_rs::Client> {
    Arc::new(topk_rs::Client::new({
        let mut client = ClientConfig::new(api_key, region)
            .with_https(https)
            .with_host(host);

        if let Some(retry_config) = retry_config {
            client = client.with_retry_config(retry_config.into());
        }

        client
    }))
}

#[derive(IntoPyObject)]
pub struct Document(pub(crate) HashMap<String, NativeValue>);

impl From<topk_rs::proto::v1::data::Document> for Document {
    fn from(doc: topk_rs::proto::v1::data::Document) -> Self {
        Document(doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

impl From<HashMap<String, topk_rs::proto::v1::data::Value>> for Document {
    fn from(doc: HashMap<String, topk_rs::proto::v1::data::Value>) -> Self {
        Document(doc.into_iter().map(|(k, v)| (k, v.into())).collect())
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: Option<usize>,

    /// Total timeout for the retry chain (milliseconds)
    pub timeout: Option<u64>,

    /// Backoff configuration
    pub backoff: Option<BackoffConfig>,
}

#[pymethods]
impl RetryConfig {
    #[new]
    pub fn new(
        max_retries: Option<usize>,
        timeout: Option<u64>,
        backoff: Option<BackoffConfig>,
    ) -> Self {
        Self {
            max_retries,
            timeout,
            backoff,
        }
    }
}

pub struct NativeRetryConfig {
    pub(crate) config: Option<RetryConfig>,
}

impl<'py> FromPyObject<'py> for NativeRetryConfig {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.downcast_exact::<RetryConfig>().is_ok() {
            return Ok(NativeRetryConfig {
                config: Some(obj.extract::<RetryConfig>()?),
            });
        }

        match obj.downcast_exact::<PyDict>() {
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
                    .map(|v| v.extract::<NativeBackoffConfig>())
                    .transpose()?;

                let backoff = backoff.map(|b| b.config.unwrap());

                Ok(NativeRetryConfig {
                    config: Some(RetryConfig {
                        max_retries,
                        timeout,
                        backoff,
                    }),
                })
            }
            _ => Err(PyTypeError::new_err(
                "`RetryConfig` must be a dict or a `RetryConfig` instance",
            )),
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

#[pyclass]
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Base for the backoff
    pub base: Option<u32>,

    /// Initial backoff (milliseconds)
    pub init_backoff: Option<u64>,

    /// Maximum backoff (milliseconds)
    pub max_backoff: Option<u64>,
}

#[pymethods]
impl BackoffConfig {
    #[new]
    pub fn new(base: Option<u32>, init_backoff: Option<u64>, max_backoff: Option<u64>) -> Self {
        Self {
            base,
            init_backoff,
            max_backoff,
        }
    }
}

pub struct NativeBackoffConfig {
    pub(crate) config: Option<BackoffConfig>,
}

impl<'py> FromPyObject<'py> for NativeBackoffConfig {
    fn extract_bound(obj: &Bound<'py, PyAny>) -> PyResult<Self> {
        if obj.downcast_exact::<BackoffConfig>().is_ok() {
            return Ok(NativeBackoffConfig {
                config: Some(obj.extract::<BackoffConfig>()?),
            });
        }

        match obj.downcast_exact::<PyDict>() {
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

                Ok(NativeBackoffConfig {
                    config: Some(BackoffConfig {
                        base,
                        init_backoff,
                        max_backoff,
                    }),
                })
            }
            _ => Err(PyTypeError::new_err(
                "`BackoffConfig` must be a dict or a `BackoffConfig` instance",
            )),
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
