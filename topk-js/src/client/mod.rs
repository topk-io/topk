use std::{sync::Arc, time::Duration};

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
    pub retry_config: Option<RetryConfig>,
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

        if let Some(retry_config) = config.retry_config {
            rs_config = rs_config.with_retry_config(retry_config.into());
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

#[napi(object)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: Option<u32>,

    /// Total timeout for the retry chain (milliseconds)
    pub timeout: Option<u32>,

    /// Backoff configuration
    pub backoff: Option<BackoffConfig>,
}

impl Into<topk_rs::retry::RetryConfig> for RetryConfig {
    fn into(self) -> topk_rs::retry::RetryConfig {
        topk_rs::retry::RetryConfig {
            max_retries: self
                .max_retries
                .unwrap_or(topk_rs::defaults::RETRY_MAX_RETRIES as u32)
                as usize,
            timeout: Duration::from_millis(
                self.timeout
                    .unwrap_or(topk_rs::defaults::RETRY_TIMEOUT as u32) as u64,
            ),
            backoff: self.backoff.map(|b| b.into()).unwrap_or_default(),
        }
    }
}

#[napi(object)]
pub struct BackoffConfig {
    /// Base for the backoff
    pub base: Option<u32>,

    /// Initial backoff (milliseconds)
    pub init_backoff: Option<u32>,

    /// Maximum backoff (milliseconds)
    pub max_backoff: Option<u32>,
}

impl Into<topk_rs::retry::BackoffConfig> for BackoffConfig {
    fn into(self) -> topk_rs::retry::BackoffConfig {
        topk_rs::retry::BackoffConfig {
            base: self.base.unwrap_or(topk_rs::defaults::RETRY_BACKOFF_BASE),
            init_backoff: Duration::from_millis(
                self.init_backoff
                    .unwrap_or(topk_rs::defaults::RETRY_BACKOFF_INIT as u32) as u64,
            ),
            max_backoff: Duration::from_millis(
                self.max_backoff
                    .unwrap_or(topk_rs::defaults::RETRY_BACKOFF_MAX as u32) as u64,
            ),
        }
    }
}
