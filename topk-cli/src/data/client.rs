use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::config::Config;
use crate::endpoint::{DataEndpoint, ManagementEndpoint};
use crate::management::{Client as ManagementClient, ProjectToken};

#[derive(Clone)]
pub struct DataClient(Client);

impl DataClient {
    pub fn new(endpoint: DataEndpoint) -> Result<Self> {
        let region = endpoint.region.as_deref().context(
            "--region is required (or set TOPK_REGION). \
             List available regions at https://docs.topk.io/regions",
        )?;
        let config = match (endpoint.api_key, endpoint.project_id) {
            // Clap rejects the combinations; this guards direct construction.
            (Some(_), Some(_)) => bail!("--project-id cannot be combined with an API key"),
            (None, None) => bail!("--api-key (or set TOPK_API_KEY) or --project-id is required"),
            (Some(api_key), None) => ClientConfig::new(api_key, region),
            (None, Some(project_id)) => ClientConfig::default()
                .with_region(region)
                .with_interceptor(Arc::new(ProjectToken::new(
                    ManagementClient::new(ManagementEndpoint {
                        host: endpoint.host.clone(),
                        oauth: endpoint.oauth.clone(),
                    })?,
                    Config::new(endpoint.oauth, Config::dir()?),
                    project_id,
                ))),
        };
        // A batch tool rides out `SlowDown`: retries never run out, an hour of
        // continuous throttling fails the request, and `--resume` picks up.
        Ok(Self(Client::new(
            config
                .with_host(endpoint.host.host)
                .with_https(endpoint.host.https)
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
