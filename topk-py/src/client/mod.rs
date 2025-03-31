use pyo3::prelude::*;
use std::sync::Arc;
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
    #[pyo3(signature = (api_key, region, host="topk.io".into(), https=true))]
    pub fn new(api_key: String, region: String, host: String, https: bool) -> Self {
        let runtime = Arc::new(Runtime::new().expect("failed to create runtime"));

        let client = Arc::new(topk_rs::Client::new(
            ClientConfig::new(api_key, region)
                .with_https(https)
                .with_host(host),
        ));

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
