use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use tonic::transport::{ClientTlsConfig, Endpoint};

use crate::client::AsyncInterceptor;
use crate::Error;

use super::retry::RetryConfig;

#[derive(Clone)]
pub struct ClientConfig {
    /// Topk region. `None` is the global control plane, which has no region label.
    region: Option<String>,

    /// Topk host (e.g. "topk.io")
    host: String,

    /// Whether to use HTTPS
    https: bool,

    /// Headers
    headers: HashMap<&'static str, String>,

    /// Retry config
    retry_config: RetryConfig,

    /// Custom interceptor
    interceptor: Option<Arc<dyn AsyncInterceptor>>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            region: None,
            host: "topk.io".to_string(),
            https: true,
            headers: HashMap::from([
                // Add SDK version
                ("x-topk-sdk-version", env!("CARGO_PKG_VERSION").to_string()),
            ]),
            retry_config: RetryConfig::default(),
            interceptor: None,
        }
    }
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self::default()
            .with_region(region)
            // Add API key
            .with_headers([("authorization", format!("Bearer {}", api_key.into()))])
    }

    // Getters

    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
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

    pub fn interceptor(&self) -> Option<&Arc<dyn AsyncInterceptor>> {
        self.interceptor.as_ref()
    }

    // Setters

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

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

    pub fn with_interceptor(mut self, interceptor: Arc<dyn AsyncInterceptor>) -> Self {
        self.interceptor = Some(interceptor);
        self
    }

    /// Builds [`Endpoint`] from the client config.
    pub fn endpoint(&self) -> Result<Endpoint, Error> {
        let protocol = if self.https() { "https" } else { "http" };
        let uri = match self.region() {
            Some(region) => format!("{}://{}.api.{}", protocol, region, self.host()),
            None => format!("{}://api.{}", protocol, self.host()),
        };
        let mut endpoint = Endpoint::from_str(&uri)?;
        if self.https() {
            endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?;
        }
        Ok(endpoint)
    }
}
