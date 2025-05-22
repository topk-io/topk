use std::collections::HashMap;

use super::retry::RetryConfig;

#[derive(Clone)]
pub struct ClientConfig {
    /// Topk region
    pub(crate) region: String,

    /// Topk host (e.g. "topk.io")
    pub(crate) host: String,

    /// Whether to use HTTPS
    pub(crate) https: bool,

    /// Headers
    pub(crate) headers: HashMap<&'static str, String>,

    /// Retry config
    pub(crate) retry_config: RetryConfig,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            host: "topk.io".to_string(),
            https: true,
            headers: HashMap::from([("authorization", format!("Bearer {}", api_key.into()))]),
            retry_config: RetryConfig::default(),
        }
    }

    /// Getters
    pub fn endpoint(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };

        format!("{}://{}.api.{}", protocol, self.region, self.host)
    }

    pub fn headers(&self) -> HashMap<&'static str, String> {
        self.headers.clone()
    }

    /// Setters
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_https(mut self, https: bool) -> Self {
        self.https = https;
        self
    }

    pub fn with_headers(mut self, headers: impl Into<HashMap<&'static str, String>>) -> Self {
        self.headers.extend(headers.into());
        self
    }

    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }
}
