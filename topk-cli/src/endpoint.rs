use std::fmt;
use std::time::Duration;

use anyhow::{Context, Result};
use tonic::transport::{ClientTlsConfig, Endpoint as GrpcEndpoint};

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::auth::{Auth, Config};
use crate::config;
use crate::management::Client as ManagementClient;

#[derive(clap::Args, Clone, Debug)]
pub struct Host {
    /// API domain
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
        global = true,
        hide = true
    )]
    pub host: String,

    /// Connect over HTTPS (default: true; --https false for a plaintext endpoint)
    #[arg(
        long,
        env = "TOPK_HTTPS",
        default_value = "true",
        num_args = 0..=1,
        default_missing_value = "true",
        global = true,
        hide = true
    )]
    pub https: bool,
}

#[derive(clap::Args, Clone)]
pub struct DataEndpoint {
    /// TopK API key
    #[arg(
        long,
        env = "TOPK_API_KEY",
        global = true,
        hide_env_values = true,
        help_heading = "Connection options"
    )]
    pub api_key: Option<String>,

    /// Region to read and write; list available regions at https://docs.topk.io/regions
    #[arg(
        long,
        env = "TOPK_REGION",
        global = true,
        help_heading = "Connection options"
    )]
    pub region: Option<String>,

    #[command(flatten)]
    pub host: Host,
}

impl fmt::Debug for DataEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataEndpoint")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("region", &self.region)
            .field("host", &self.host)
            .finish()
    }
}

impl DataEndpoint {
    pub fn client(&self) -> Result<Client> {
        let api_key =
            self.api_key.as_deref().filter(|v| !v.is_empty()).context(
                "API key not set. Set TOPK_API_KEY environment variable or pass --api-key.",
            )?;
        let region = self.region.as_deref().filter(|v| !v.is_empty()).context(
            "--region is required (or set TOPK_REGION). \
             List available regions at https://docs.topk.io/regions",
        )?;
        // A batch tool rides out `SlowDown`: retries never run out, an hour of
        // continuous throttling fails the request, and `--resume` picks up.
        Ok(Client::new(
            ClientConfig::new(api_key, region)
                .with_host(&self.host.host)
                .with_https(self.host.https)
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

#[derive(clap::Args, Clone, Debug)]
pub struct ManagementEndpoint {
    #[command(flatten)]
    pub host: Host,

    #[command(flatten)]
    pub auth: Config,
}

impl ManagementEndpoint {
    pub fn auth(&self) -> Result<Auth> {
        Auth::new(&self.auth, config::dir().context("no config directory")?)
    }

    pub fn client(&self) -> Result<ManagementClient> {
        let Host { host, https } = &self.host;
        let protocol = if *https { "https" } else { "http" };
        let mut endpoint = GrpcEndpoint::from_shared(format!("{protocol}://api.{host}"))?;
        if *https {
            endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?;
        }
        Ok(ManagementClient::new(endpoint, self.auth()?))
    }
}
