use std::collections::HashMap;
use std::str::FromStr;

use tonic::transport::Endpoint;

use crate::Error;

use super::retry::RetryConfig;

#[derive(Clone)]
pub struct ClientConfig {
    /// Topk region
    region: String,

    /// Topk host (e.g. "topk.io")
    host: String,

    /// Whether to use HTTPS
    https: bool,

    /// Headers
    headers: HashMap<&'static str, String>,

    /// Retry config
    retry_config: RetryConfig,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            host: "topk.io".to_string(),
            https: true,
            headers: HashMap::from([
                // Add API key
                ("authorization", format!("Bearer {}", api_key.into())),
                // Add SDK version
                ("x-topk-sdk-version", env!("CARGO_PKG_VERSION").to_string()),
            ]),
            retry_config: RetryConfig::default(),
        }
    }

    // Getters

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn https(&self) -> bool {
        self.https
    }

    pub fn headers(&self) -> &HashMap<&'static str, String> {
        &self.headers
    }

    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    // Setters

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_https(mut self, https: bool) -> Self {
        self.https = https;
        self
    }

    pub fn with_headers(
        mut self,
        headers: impl IntoIterator<Item = (&'static str, impl Into<String>)>,
    ) -> Self {
        self.headers
            .extend(headers.into_iter().map(|(key, value)| (key, value.into())));
        self
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Builds [`Endpoint`] from the client config.
    pub fn endpoint(&self) -> Result<Endpoint, Error> {
        let protocol = if self.https() { "https" } else { "http" };
        let uri = match self.region() {
            "global" => format!("{}://api.{}", protocol, self.host()),
            _ => format!("{}://{}.api.{}", protocol, self.region(), self.host()),
        };
        Ok(Endpoint::from_str(&uri)?)
    }
}
