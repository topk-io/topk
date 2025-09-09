use std::sync::Arc;

use pyo3::{prelude::*, PyResult};
use topk_rs::ClientConfig;

use crate::client::{
    r#async::{collection::AsyncCollectionClient, collections::AsyncCollectionsClient},
    RetryConfig,
};

mod collection;
mod collections;

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
        let client = Arc::new(topk_rs::Client::new({
            let mut client = ClientConfig::new(api_key, region)
                .with_https(https)
                .with_host(host);

            if let Some(retry_config) = retry_config {
                client = client.with_retry_config(retry_config.into());
            }

            client
        }));

        Self { client }
    }

    pub fn collection(&self, collection: String) -> PyResult<AsyncCollectionClient> {
        Ok(AsyncCollectionClient::new(self.client.clone(), collection))
    }

    pub fn collections(&self) -> PyResult<AsyncCollectionsClient> {
        Ok(AsyncCollectionsClient::new(self.client.clone()))
    }
}
