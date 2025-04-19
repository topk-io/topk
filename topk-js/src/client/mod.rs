use std::sync::Arc;

use collection::CollectionClient;
use collections::CollectionsClient;
use napi_derive::napi;
use topk_rs::{Client as RsClient, ClientConfig as RsClientConfig};

pub mod collection;
pub mod collections;

#[napi(object)]
pub struct ClientConfig {
    pub api_key: String,
    pub region: String,
    pub host: Option<String>,
    pub https: Option<bool>,
}

#[napi]
pub struct Client {
    client: Arc<RsClient>,
}

#[napi]
impl Client {
    #[napi(constructor)]
    pub fn new(config: ClientConfig) -> Self {
        let mut rs_config = RsClientConfig::new(config.api_key, config.region);

        if let Some(host_value) = config.host {
            rs_config = rs_config.with_host(host_value);
        }

        if let Some(https_value) = config.https {
            rs_config = rs_config.with_https(https_value);
        }

        let rs_client = RsClient::new(rs_config);

        let client = Arc::new(rs_client);

        Self { client }
    }

    #[napi]
    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(self.client.clone())
    }

    #[napi]
    pub fn collection(&self, name: String) -> CollectionClient {
        CollectionClient::new(self.client.clone(), name)
    }
}
