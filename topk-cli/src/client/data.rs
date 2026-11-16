use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::client::{AccessTokenProvider, ManagementClient};
use crate::config::Config;
use crate::ProjectId;

impl Config {
    pub fn data_client(&self, project_id: ProjectId, region: &str) -> Result<Client> {
        let client = ClientConfig::default()
            .with_region(region)
            .with_interceptor(Arc::new(AccessTokenProvider::new(
                ManagementClient::new(self.clone())?,
                self.clone(),
                project_id,
            )));
        // A batch tool rides out transient errors: retries never run out, an hour of
        // continuous failures fails the request, and `--resume` picks up.
        Ok(Client::new(
            client
                .with_host(self.host())
                .with_https(self.https())
                .with_retry_config(RetryConfig {
                    max_retries: usize::MAX,
                    timeout: Duration::from_secs(60 * 60),
                    backoff: BackoffConfig {
                        init_backoff: Duration::from_millis(250),
                        ..BackoffConfig::default()
                    },
                }),
        ))
    }
}
