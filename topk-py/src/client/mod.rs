use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::IntoPyObject;
use std::{collections::HashMap, sync::Arc, time::Duration};
use topk_rs::ClientConfig;

use crate::data::dataset::Dataset;
use crate::data::value::NativeValue;
use crate::data::value::Value;
use pyo3::PyClass;

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
    pub(crate) config: RetryConfig,
}

impl<'a, 'py> FromPyObject<'a, 'py> for NativeRetryConfig {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if obj.cast_exact::<RetryConfig>().is_ok() {
            return Ok(NativeRetryConfig {
                config: obj.extract::<RetryConfig>()?,
            });
        }

        match obj.cast_exact::<PyDict>() {
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

                Ok(NativeRetryConfig {
                    config: RetryConfig {
                        max_retries,
                        timeout,
                        backoff: backoff.map(|b| b.config),
                    },
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
    #[pyo3(signature = (base=None, init_backoff=None, max_backoff=None))]
    pub fn new(base: Option<u32>, init_backoff: Option<u64>, max_backoff: Option<u64>) -> Self {
        Self {
            base,
            init_backoff,
            max_backoff,
        }
    }
}

pub struct NativeBackoffConfig {
    pub(crate) config: BackoffConfig,
}

impl<'a, 'py> FromPyObject<'a, 'py> for NativeBackoffConfig {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if obj.cast_exact::<BackoffConfig>().is_ok() {
            return Ok(NativeBackoffConfig {
                config: obj.extract::<BackoffConfig>()?,
            });
        }

        match obj.cast_exact::<PyDict>() {
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
                    config: BackoffConfig {
                        base,
                        init_backoff,
                        max_backoff,
                    },
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

/// Configuration for polling when waiting for a handle to be processed.
#[pyclass]
#[derive(Debug, Clone)]
pub struct WaitConfig {
    /// How often to poll for the handle status (seconds). Default is 5.
    pub frequency_secs: Option<u64>,
    /// Maximum time to wait before returning a timeout error (seconds). Default is 300.
    pub timeout_secs: Option<u64>,
}

#[pymethods]
impl WaitConfig {
    #[new]
    #[pyo3(signature = (frequency_secs=None, timeout_secs=None))]
    pub fn new(frequency_secs: Option<u64>, timeout_secs: Option<u64>) -> Self {
        Self {
            frequency_secs,
            timeout_secs,
        }
    }
}

impl Into<topk_rs::client::WaitConfig> for WaitConfig {
    fn into(self) -> topk_rs::client::WaitConfig {
        topk_rs::client::WaitConfig {
            frequency: Duration::from_secs(self.frequency_secs.unwrap_or(5)),
            timeout: Duration::from_secs(self.timeout_secs.unwrap_or(300)),
        }
    }
}

pub struct NativeWaitConfig {
    pub(crate) config: WaitConfig,
}

impl<'a, 'py> FromPyObject<'a, 'py> for NativeWaitConfig {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if obj.cast_exact::<WaitConfig>().is_ok() {
            return Ok(NativeWaitConfig {
                config: obj.extract::<WaitConfig>()?,
            });
        }

        match obj.cast_exact::<PyDict>() {
            Ok(dict) => {
                let frequency_secs = dict
                    .get_item("frequency_secs")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?;

                let timeout_secs = dict
                    .get_item("timeout_secs")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?;

                Ok(NativeWaitConfig {
                    config: WaitConfig {
                        frequency_secs,
                        timeout_secs,
                    },
                })
            }
            _ => Err(PyTypeError::new_err(
                "`WaitConfig` must be a dict or a `WaitConfig` instance",
            )),
        }
    }
}

// Python response base class and conversion helper (shared by datasets and dataset APIs)

#[pyclass(subclass)]
pub struct Response {
    #[pyo3(get)]
    pub request_id: Option<String>,
}

#[pymethods]
impl Response {
    #[new]
    fn new(request_id: Option<String>) -> Self {
        Self { request_id }
    }
}

/// Convert `topk_rs::Response<Proto>` into a Python response object.
/// Extracts `request_id` once and builds `(Sub, Response)` for PyO3.
pub(crate) fn into_py_response<Proto, Sub, F>(
    py: Python<'_>,
    response: topk_rs::client::Response<Proto>,
    f: F,
) -> PyResult<Py<Sub>>
where
    F: FnOnce(Proto) -> PyResult<Sub>,
    Sub: PyClass<BaseType = Response>,
{
    let request_id = response.request_id().map(|r| r.as_str().to_string());
    let inner = response.into_inner();
    let sub = f(inner)?;
    Py::new(py, (sub, Response { request_id }))
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct GetDatasetResponse {
    #[pyo3(get)]
    pub dataset: Dataset,
}

#[pymethods]
impl GetDatasetResponse {
    #[new]
    fn new(dataset: Dataset, request_id: Option<String>) -> (Self, Response) {
        (Self { dataset }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct ListDatasetsResponse {
    #[pyo3(get)]
    pub datasets: Vec<Dataset>,
}

#[pymethods]
impl ListDatasetsResponse {
    #[new]
    fn new(datasets: Vec<Dataset>, request_id: Option<String>) -> (Self, Response) {
        (Self { datasets }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct CreateDatasetResponse {
    #[pyo3(get)]
    pub dataset: Dataset,
}

#[pymethods]
impl CreateDatasetResponse {
    #[new]
    fn new(dataset: Dataset, request_id: Option<String>) -> (Self, Response) {
        (Self { dataset }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct DeleteDatasetResponse;

#[pymethods]
impl DeleteDatasetResponse {
    #[new]
    fn new(request_id: Option<String>) -> (Self, Response) {
        (Self, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct UpsertResponse {
    #[pyo3(get)]
    pub handle: String,
}

#[pymethods]
impl UpsertResponse {
    #[new]
    fn new(handle: String, request_id: Option<String>) -> (Self, Response) {
        (Self { handle }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct GetMetadataResponse {
    /// Map from document ID to metadata fields.
    #[pyo3(get)]
    pub docs: HashMap<String, HashMap<String, Value>>,
}

#[pymethods]
impl GetMetadataResponse {
    #[new]
    fn new(
        docs: HashMap<String, HashMap<String, Value>>,
        request_id: Option<String>,
    ) -> (Self, Response) {
        (Self { docs }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct UpdateMetadataResponse {
    #[pyo3(get)]
    pub handle: String,
}

#[pymethods]
impl UpdateMetadataResponse {
    #[new]
    fn new(handle: String, request_id: Option<String>) -> (Self, Response) {
        (Self { handle }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct DeleteFileResponse {
    #[pyo3(get)]
    pub handle: String,
}

#[pymethods]
impl DeleteFileResponse {
    #[new]
    fn new(handle: String, request_id: Option<String>) -> (Self, Response) {
        (Self { handle }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[pyclass(extends=Response)]
#[derive(Debug)]
pub struct CheckHandleResponse {
    #[pyo3(get)]
    pub processed: bool,
}

#[pymethods]
impl CheckHandleResponse {
    #[new]
    fn new(processed: bool, request_id: Option<String>) -> (Self, Response) {
        (Self { processed }, Response { request_id })
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}
