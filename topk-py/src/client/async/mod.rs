use std::sync::Arc;

use pyo3::{prelude::*, PyResult};

use crate::client::{topk_client, RetryConfig};

mod collection;
mod collections;

pub use collection::AsyncCollectionClient;
pub use collections::AsyncCollectionsClient;

#[pyclass]
pub struct AsyncClient {
    client: Arc<topk_rs::Client>,
}

#[pymethods]
impl AsyncClient {
    #[new]
    #[pyo3(signature = (api_key, region, host="topk.io".into(), https=true, retry_config=None))]
    pub fn new(
        api_key: String,
        region: String,
        host: String,
        https: bool,
        retry_config: Option<RetryConfig>,
    ) -> Self {
        let client = topk_client(api_key, region, host, https, retry_config);

        Self { client }
    }

    pub fn collection(&self, collection: String) -> PyResult<AsyncCollectionClient> {
        Ok(AsyncCollectionClient::new(
            self.client.clone(),
            Arc::new(collection),
        ))
    }

    pub fn collections(&self) -> PyResult<AsyncCollectionsClient> {
        Ok(AsyncCollectionsClient::new(self.client.clone()))
    }
}
