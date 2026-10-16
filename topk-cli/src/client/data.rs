use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::client::{AccessTokenProvider, ManagementClient};
use crate::config::Config;
use crate::endpoint::{Credentials, DataEndpoint};

#[derive(Clone)]
pub struct DataClient(Client);

impl DataClient {
    pub fn new(config: &Config, endpoint: DataEndpoint) -> Result<Self> {
        let region = endpoint.region()?;
        let client = match endpoint.credentials(config)? {
            Credentials::ApiKey(api_key) => ClientConfig::new(api_key, region),
            Credentials::Project(project_id) => ClientConfig::default()
                .with_region(region)
                .with_interceptor(Arc::new(AccessTokenProvider::new(
                    ManagementClient::new(config.clone())?,
                    config.clone(),
                    project_id,
                ))),
        };
        // A batch tool rides out transient errors: retries never run out, an hour of
        // continuous failures fails the request, and `--resume` picks up.
        Ok(Self(Client::new(
            client
                .with_host(config.host())
                .with_https(config.https())
                .with_retry_config(RetryConfig {
                    max_retries: usize::MAX,
                    timeout: Duration::from_secs(60 * 60),
                    backoff: BackoffConfig {
                        init_backoff: Duration::from_millis(250),
                        ..BackoffConfig::default()
                    },
                }),
        )))
    }
}

impl Deref for DataClient {
    type Target = Client;

    fn deref(&self) -> &Client {
        &self.0
    }
}
