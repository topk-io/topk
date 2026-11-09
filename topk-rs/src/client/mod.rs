use std::sync::Arc;
use std::time::Duration;

use tokio::sync::OnceCell;
use tonic::transport::Channel;

mod collections;
pub use collections::CollectionsClient;

mod collection;
pub use collection::CollectionClient;
pub use collection::DocumentStream;

mod config;
pub use config::ClientConfig;

pub mod retry;

mod transport;
pub use transport::AsyncInterceptor;

// (client) max message size for all requests
pub const MAX_DECODING_MESSAGE_SIZE: usize = 512 * 1024 * 1024; // 512MB
pub const MAX_ENCODING_MESSAGE_SIZE: usize = 512 * 1024 * 1024; // 512MB

// request config
pub const MAX_HEADER_LIST_SIZE: u32 = 1024 * 64; // 64KB

// (client) keepalive config; GCP's load balancer sends GOAWAY too_many_pings at a 10s interval
pub const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(60);
pub const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(30);

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

    pub fn collection(&self, name: impl Into<String>) -> CollectionClient {
        // Collection services expect `x-topk-collection` header to be set.
        let config = self
            .config
            .clone()
            .with_headers([("x-topk-collection", name)]);

        CollectionClient::new(config, self.channel.clone(), self.channel.clone())
    }
}

// Macro for instantiating and connecting a client
#[macro_export]
macro_rules! create_client {
    ($client:ident, $channel:expr, $config:expr) => {
        async {
            use crate::client;
            use crate::Error;

            // Lazily connect the channel on first use
            let channel = $channel
                .get_or_try_init(|| async {
                    Ok::<_, Error>(
                        $config
                            .endpoint()?
                            // Send keepalive pings even while the connection is idle.
                            .keep_alive_while_idle(true)
                            // Use keepalive to detect stalled connections.
                            .http2_keep_alive_interval(client::KEEP_ALIVE_INTERVAL)
                            .keep_alive_timeout(client::KEEP_ALIVE_TIMEOUT)
                            // Set max header list size to 64KB
                            .http2_max_header_list_size(client::MAX_HEADER_LIST_SIZE)
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

            // Create transport
            let transport = client::transport::Transport::new(channel.clone(), &$config)?;

            // Build client
            let client = $client::new(transport)
                .max_decoding_message_size(client::MAX_DECODING_MESSAGE_SIZE)
                .max_encoding_message_size(client::MAX_ENCODING_MESSAGE_SIZE);

            Result::<_, Error>::Ok(client)
        }
    };
}
