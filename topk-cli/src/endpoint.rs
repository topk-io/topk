use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tonic::transport::{ClientTlsConfig, Endpoint as GrpcEndpoint};

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::auth::{Auth, Config};
use crate::config;
use crate::management::{Client as ManagementClient, ProjectTokenInterceptor, ProjectTokens};

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

    /// Project to access with your login instead of an API key
    #[arg(long, global = true, help_heading = "Connection options")]
    pub project_id: Option<String>,

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

    #[command(flatten)]
    pub auth: Config,
}

impl fmt::Debug for DataEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataEndpoint")
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .field("project_id", &self.project_id)
            .field("region", &self.region)
            .field("host", &self.host)
            .field("auth", &self.auth)
            .finish()
    }
}

impl DataEndpoint {
    pub fn mgmt(&self) -> ManagementEndpoint {
        ManagementEndpoint {
            host: self.host.clone(),
            auth: self.auth.clone(),
        }
    }

    pub fn client(&self) -> Result<Client> {
        let region = self.region.as_deref().filter(|v| !v.is_empty()).context(
            "--region is required (or set TOPK_REGION). \
             List available regions at https://docs.topk.io/regions",
        )?;
        let project_id = self.project_id.as_deref().filter(|v| !v.is_empty());
        let config = match (self.api_key.clone().filter(|v| !v.is_empty()), project_id) {
            (Some(_), Some(_)) => bail!(
                "--project-id cannot be combined with an API key (--api-key or TOPK_API_KEY). \
                 Unset TOPK_API_KEY to use your login, or drop --project-id to use the API key."
            ),
            (Some(api_key), None) => ClientConfig::new(api_key, region),
            (None, Some(project_id)) => ClientConfig::default()
                .with_region(region)
                .with_interceptor(Arc::new(ProjectTokenInterceptor::new(
                    Arc::new(ProjectTokens::new(
                        self.mgmt().client()?,
                        &self.mgmt().auth()?,
                    )),
                    project_id.to_owned(),
                ))),
            (None, None) => bail!("--project-id is required"),
        };
        // A batch tool rides out `SlowDown`: retries never run out, an hour of
        // continuous throttling fails the request, and `--resume` picks up.
        Ok(Client::new(
            config
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
