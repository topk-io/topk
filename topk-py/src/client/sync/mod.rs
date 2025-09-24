mod collection;
mod collections;
mod runtime;

use std::sync::Arc;

pub use collection::CollectionClient;
pub use collections::CollectionsClient;

use crate::client::{sync::runtime::Runtime, topk_client, RetryConfig};

use pyo3::{pyclass, pymethods, PyResult};

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

        let client = topk_client(api_key, region, host, https, retry_config);

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
