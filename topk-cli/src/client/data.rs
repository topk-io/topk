use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::client::{AccessTokenInterceptor, ManagementClient};
use crate::config::Config;
use crate::endpoint::DataEndpoint;
use crate::host::Host;

#[derive(Clone)]
pub struct DataClient(Client);

impl DataClient {
    pub fn new(config: &Config, endpoint: DataEndpoint) -> Result<Self> {
        let region = endpoint.region.as_deref().context(
            "--region is required (or set TOPK_REGION). \
             List available regions at https://docs.topk.io/regions",
        )?;
        let client = match (endpoint.api_key, endpoint.project_id) {
            // Clap rejects the combinations; this guards direct construction.
            (Some(_), Some(_)) => bail!("--project-id cannot be combined with an API key"),
            (None, None) => bail!("--api-key (or set TOPK_API_KEY) or --project-id is required"),
            (Some(api_key), None) => ClientConfig::new(api_key, region),
            // `--project-id` authenticates with access tokens minted through the login.
            (None, Some(project_id)) => ClientConfig::default()
                .with_region(region)
                .with_interceptor(Arc::new(AccessTokenInterceptor::new(
                    ManagementClient::new(config.clone())?,
                    config.clone(),
                    project_id,
                ))),
        };
        let Host { host, https } = config.host();
        // A batch tool rides out `SlowDown`: retries never run out, an hour of
        // continuous throttling fails the request, and `--resume` picks up.
        Ok(Self(Client::new(
            client
                .with_host(host.clone())
                .with_https(*https)
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
