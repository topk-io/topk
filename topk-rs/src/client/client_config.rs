#[derive(Clone)]
pub struct ClientConfig {
    /// Topk region
    region: String,

    /// Topk host (e.g. "topk.io")
    host: String,

    /// Whether to use HTTPS
    https: bool,

    /// API key
    api_key: String,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            host: "topk.io".to_string(),
            https: true,
            api_key: api_key.into(),
        }
    }

    /// Getters
    pub fn endpoint(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };

        format!("{}://{}.api.{}", protocol, self.region, self.host)
    }

    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.api_key)
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
}
