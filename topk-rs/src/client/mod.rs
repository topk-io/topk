use std::sync::Arc;

use tokio::sync::OnceCell;
use tonic::transport::Channel;

mod collections;
pub use collections::CollectionsClient;

mod datasets;
pub use datasets::DatasetsClient;

mod collection;
pub use collection::CollectionClient;

mod dataset;
pub use dataset::DatasetClient;

pub mod ask;
pub mod search;

mod config;
pub use config::ClientConfig;

mod response;
pub use response::{extract_request_id, RequestId, Response};

pub mod retry;

mod interceptor;
pub use interceptor::AppendHeadersInterceptor;

// (client) max message size for all requests
pub const MAX_DECODING_MESSAGE_SIZE: usize = 512 * 1024 * 1024; // 512MB
pub const MAX_ENCODING_MESSAGE_SIZE: usize = 512 * 1024 * 1024; // 512MB

// request config
pub const TIMEOUT: u64 = 600_000; // 10 minutes
pub const MAX_HEADER_LIST_SIZE: u32 = 1024 * 64; // 64KB

// (client) retry config
pub const RETRY_TIMEOUT: u64 = 180_000; // 3 minutes
pub const RETRY_MAX_RETRIES: usize = 3; // 3 retries
pub const RETRY_BACKOFF_INIT: u64 = 100; // 100 milliseconds
pub const RETRY_BACKOFF_MAX: u64 = 10_000; // 10 seconds
pub const RETRY_BACKOFF_BASE: u32 = 2; // `Base` is the multiplier for the backoff

#[derive(Clone)]
pub struct Client {
    // Client config
    config: ClientConfig,

    // Channel (lazily connected on first request)
    channel: Arc<OnceCell<Channel>>,
}

impl Client {
    /// Creates a new client with the provided configuration.
    ///
    /// The connection is established lazily, at the first request.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            channel: Arc::new(OnceCell::new()),
        }
    }

    pub fn from_channel(config: ClientConfig, channel: Channel) -> Self {
        Self {
            config,
            channel: Arc::new(OnceCell::new_with(Some(channel))),
        }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(self.config.clone(), self.channel.clone())
    }

    pub fn datasets(&self) -> DatasetsClient {
        DatasetsClient::new(self.config.clone(), self.channel.clone())
    }

    pub fn collection(&self, name: impl Into<String>) -> CollectionClient {
        // Collection services expect `x-topk-collection` header to be set.
        let config = self
            .config
            .clone()
            .with_headers([("x-topk-collection", name)]);

        CollectionClient::new(config, self.channel.clone(), self.channel.clone())
    }

    pub fn dataset(&self, name: impl Into<String>) -> DatasetClient {
        // Dataset services expect `x-topk-dataset` header to be set.
        let config = self.config.clone().with_headers([("x-topk-dataset", name)]);

        DatasetClient::new(config, self.channel.clone(), self.channel.clone())
    }
}

// Macro for instantiating and connecting a client
#[macro_export]
macro_rules! create_client {
    ($client:ident, $channel:expr, $config:expr) => {
        async {
            use crate::client::AppendHeadersInterceptor;
            use crate::client::MAX_DECODING_MESSAGE_SIZE;
            use crate::client::MAX_ENCODING_MESSAGE_SIZE;
            use crate::client::MAX_HEADER_LIST_SIZE;
            use crate::client::TIMEOUT;
            use crate::Error;

            // Lazily connect the channel on first use
            let channel = $channel
                .get_or_try_init(|| async {
                    Ok::<_, Error>(
                        $config
                            .endpoint()?
                            .tls_config(
                                tonic::transport::ClientTlsConfig::new().with_native_roots(),
                            )?
                            // Do not close idle connections so they can be reused
                            .keep_alive_while_idle(true)
                            // Set max header list size to 64KB
                            .http2_max_header_list_size(MAX_HEADER_LIST_SIZE)
                            // Request timeout
                            .timeout(std::time::Duration::from_millis(TIMEOUT))
                            // Disable Nagle's algorithm
                            .tcp_nodelay(true)
                            // Disable adaptive window
                            .http2_adaptive_window(false)
                            .initial_stream_window_size(8 * 1024 * 1024) // 8MB
                            .initial_connection_window_size(32 * 1024 * 1024) // 32MB
                            // Connect
                            .connect()
                            .await?,
                    )
                })
                .await?;

            // Build interceptor
            let interceptor = AppendHeadersInterceptor::new($config.headers().clone())?;

            // Build client
            let client = $client::with_interceptor(channel.clone(), interceptor)
                .max_decoding_message_size(MAX_DECODING_MESSAGE_SIZE)
                .max_encoding_message_size(MAX_ENCODING_MESSAGE_SIZE);

            Result::<_, Error>::Ok(client)
        }
    };
}
