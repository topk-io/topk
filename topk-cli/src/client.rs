use std::sync::Arc;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use tonic::transport::{ClientTlsConfig, Endpoint as GrpcEndpoint};

use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig};

use crate::auth::Auth;
use crate::config;
use crate::endpoint::{DataEndpoint, Host, ManagementEndpoint};
use crate::management::{Client as ManagementClient, ProjectToken};

pub fn auth(endpoint: &ManagementEndpoint) -> Result<Auth> {
    Auth::new(
        &endpoint.auth,
        config::dir().context("no config directory")?,
    )
}

pub fn management_client(endpoint: &ManagementEndpoint) -> Result<ManagementClient> {
    let Host { host, https } = &endpoint.host;
    let protocol = if *https { "https" } else { "http" };
    let mut grpc = GrpcEndpoint::from_shared(format!("{protocol}://api.{host}"))?;
    if *https {
        grpc = grpc.tls_config(ClientTlsConfig::new().with_native_roots())?;
    }
    Ok(ManagementClient::new(grpc, auth(endpoint)?))
}

pub fn data_client(endpoint: &DataEndpoint) -> Result<Client> {
    let region = endpoint
        .region
        .as_deref()
        .filter(|v| !v.is_empty())
        .context(
            "--region is required (or set TOPK_REGION). \
         List available regions at https://docs.topk.io/regions",
        )?;
    let config = match (
        endpoint.api_key.clone().filter(|v| !v.is_empty()),
        &endpoint.project_id,
    ) {
        (Some(_), Some(_)) => bail!(
            "--project-id cannot be combined with an API key (--api-key or TOPK_API_KEY). \
             Unset TOPK_API_KEY to use your login, or drop --project-id to use the API key."
        ),
        (Some(api_key), None) => ClientConfig::new(api_key, region),
        (None, Some(project_id)) => {
            let mgmt = ManagementEndpoint::from(endpoint);
            ClientConfig::default()
                .with_region(region)
                .with_interceptor(Arc::new(ProjectToken::new(
                    management_client(&mgmt)?,
                    auth(&mgmt)?.sessions().clone(),
                    project_id.clone(),
                )))
        }
        (None, None) => bail!("--project-id is required"),
    };
    // A batch tool rides out `SlowDown`: retries never run out, an hour of
    // continuous throttling fails the request, and `--resume` picks up.
    Ok(Client::new(
        config
            .with_host(&endpoint.host.host)
            .with_https(endpoint.host.https)
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
