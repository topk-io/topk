use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig, Error};

#[derive(clap::Args, Clone, Default)]
pub struct Endpoint {
    /// TopK API key (or run `topk login`)
    #[arg(
        long,
        env = "TOPK_API_KEY",
        global = true,
        hide_env_values = true,
        help_heading = "Global options"
    )]
    pub api_key: Option<String>,

    /// Region to read and write; list available regions at https://docs.topk.io/regions
    #[arg(
        long,
        env = "TOPK_REGION",
        global = true,
        help_heading = "Global options"
    )]
    pub region: Option<String>,

    /// API domain; the endpoint is <REGION>.api.<HOST>
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
        global = true,
        help_heading = "Global options"
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
        help_heading = "Global options"
    )]
    pub https: bool,
}

impl Endpoint {
    /// `--api-key`/`TOPK_API_KEY`, else the saved login. A set-but-empty env
    /// var (`TOPK_API_KEY=`) reads as unset.
    pub fn api_key(&self) -> Result<Option<String>, Error> {
        if let Some(key) = self.api_key.clone().filter(|v| !v.is_empty()) {
            return Ok(Some(key));
        }
        // TODO: remove stub — once `topk login` is SSO, the key comes back
        // from Auth0, not out of config.toml.
        Ok(crate::config::load()?.api_key)
    }

    pub fn client(&self) -> Result<Client, Error> {
        let api_key = self.api_key()?.ok_or_else(|| {
            Error::Unauthenticated(
                "API key not set. Run `topk login` or set TOPK_API_KEY.".to_string(),
            )
        })?;
        let region = self
            .region
            .as_deref()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                Error::InvalidArgument(
                    "--region is required (or set TOPK_REGION). \
                     List available regions at https://docs.topk.io/regions"
                        .to_string(),
                )
            })?;
        // A batch tool rides out `SlowDown`: retries never run out, an hour of
        // continuous throttling fails the request, and `--resume` picks up.
        Ok(Client::new(
            ClientConfig::new(&api_key, region)
                .with_host(&self.host)
                .with_https(self.https)
                .with_retry_config(RetryConfig {
                    max_retries: usize::MAX,
                    timeout: std::time::Duration::from_secs(60 * 60),
                    backoff: BackoffConfig {
                        init_backoff: std::time::Duration::from_millis(250),
                        ..BackoffConfig::default()
                    },
                }),
        ))
    }

    pub fn console_url(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        format!("{}://console.{}/api-key", scheme, self.host)
    }
}
