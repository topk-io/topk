use crate::error::Error;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::OnceCell;
use tonic::transport::{Channel as TonicChannel, Endpoint};

mod collections;
pub use collections::CollectionsClient;

mod collection;
pub use collection::CollectionClient;

#[derive(Debug, Clone)]
pub enum Channel {
    Endpoint(String),
    Tonic(OnceCell<TonicChannel>),
}

impl Channel {
    pub fn from_endpoint(endpoint: impl Into<String>) -> Self {
        Self::Endpoint(endpoint.into())
    }

    pub fn from_tonic(channel: TonicChannel) -> Self {
        Self::Tonic(OnceCell::from(channel))
    }

    async fn get(&self) -> Result<TonicChannel, Error> {
        match self {
            Self::Endpoint(endpoint) => Ok(Endpoint::from_str(endpoint)?
                .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())?
                // Do not close idle connections so they can be reused
                .keep_alive_while_idle(true)
                // Set max header list size to 64KB
                .http2_max_header_list_size(1024 * 64)
                .connect()
                .await?),
            Self::Tonic(cell) => match cell.get() {
                Some(channel) => Ok(channel.clone()),
                None => Err(Error::TransportChannelNotInitialized),
            },
        }
    }
}

#[derive(Clone)]
pub struct ClientConfig {
    region: String,
    host: String,
    https: bool,
    headers: HashMap<&'static str, String>,
}

impl ClientConfig {
    pub fn new(api_key: impl Into<String>, region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            host: "topk.io".to_string(),
            https: true,
            headers: HashMap::from([("authorization", format!("Bearer {}", api_key.into()))]),
        }
    }

    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn with_https(mut self, https: bool) -> Self {
        self.https = https;
        self
    }

    pub fn with_headers(mut self, headers: HashMap<&'static str, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn headers(&self) -> HashMap<&'static str, String> {
        self.headers.clone()
    }

    pub fn endpoint(&self) -> String {
        let protocol = if self.https { "https" } else { "http" };

        format!("{}://{}.api.{}", protocol, self.region, self.host)
    }
}

#[derive(Clone)]
pub struct Client {
    config: ClientConfig,
    channel: Channel,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            channel: Channel::from_endpoint(config.endpoint().clone()),
            config,
        }
    }

    pub fn from_channel(channel: TonicChannel, config: ClientConfig) -> Self {
        Self {
            config,
            channel: Channel::from_tonic(channel),
        }
    }

    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(self.config.clone(), self.channel.clone())
    }

    pub fn collection(&self, name: impl Into<String>) -> CollectionClient {
        CollectionClient::new(self.config.clone(), self.channel.clone(), name.into())
    }
}
