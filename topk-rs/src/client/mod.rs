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
pub use dataset::WaitConfig;

pub mod ask;
pub mod search;

mod config;
pub use config::ClientConfig;

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

#[derive(Clone, Default)]
struct Connection {
    control: Arc<OnceCell<Channel>>,
    read: Arc<OnceCell<Channel>>,
    write: Arc<OnceCell<Channel>>,
    ctx_read: Arc<OnceCell<Channel>>,
    ctx_write: Arc<OnceCell<Channel>>,
}

impl Connection {
    fn new(
        control: Option<Channel>,
        read: Option<Channel>,
        write: Option<Channel>,
        ctx_read: Option<Channel>,
        ctx_write: Option<Channel>,
    ) -> Self {
        Self {
            control: Arc::new(OnceCell::new_with(control)),
            read: Arc::new(OnceCell::new_with(read)),
            write: Arc::new(OnceCell::new_with(write)),
            ctx_read: Arc::new(OnceCell::new_with(ctx_read)),
            ctx_write: Arc::new(OnceCell::new_with(ctx_write)),
        }
    }
}

pub struct ClientBuilder {
    config: ClientConfig,
    control: Option<Channel>,
    read: Option<Channel>,
    write: Option<Channel>,
    ctx_read: Option<Channel>,
    ctx_write: Option<Channel>,
}

impl ClientBuilder {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            control: None,
            read: None,
            write: None,
            ctx_read: None,
            ctx_write: None,
        }
    }

    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.control = Some(channel.clone());
        self.read = Some(channel.clone());
        self.write = Some(channel.clone());
        self.ctx_read = Some(channel.clone());
        self.ctx_write = Some(channel);
        self
    }

    pub fn with_control_channel(mut self, channel: Channel) -> Self {
        self.control = Some(channel);
        self
    }

    pub fn with_read_channel(mut self, channel: Channel) -> Self {
        self.read = Some(channel);
        self
    }

    pub fn with_write_channel(mut self, channel: Channel) -> Self {
        self.write = Some(channel);
        self
    }

    pub fn with_ctx_read_channel(mut self, channel: Channel) -> Self {
        self.ctx_read = Some(channel);
        self
    }

    pub fn with_ctx_write_channel(mut self, channel: Channel) -> Self {
        self.ctx_write = Some(channel);
        self
    }

    pub fn build(self) -> Client {
        Client {
            config: self.config,
            conn: Connection::new(
                self.control,
                self.read,
                self.write,
                self.ctx_read,
                self.ctx_write,
            ),
        }
    }
}

#[derive(Clone)]
pub struct Client {
    // Client config
    config: ClientConfig,

    // Connection
    conn: Connection,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        ClientBuilder::new(config).build()
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(self.config.clone(), self.conn.control.clone())
    }

    pub fn datasets(&self) -> DatasetsClient {
        DatasetsClient::new(self.config.clone(), self.conn.control.clone())
    }

    pub fn collection(&self, name: impl Into<String>) -> CollectionClient {
        // Collection services expect `x-topk-collection` header to be set.
        let config = self
            .config
            .clone()
            .with_headers([("x-topk-collection", name)]);

        CollectionClient::new(config, self.conn.read.clone(), self.conn.write.clone())
    }

    pub fn dataset(&self, name: impl Into<String>) -> DatasetClient {
        // Dataset services expect `x-topk-dataset` header to be set.
        let config = self.config.clone().with_headers([("x-topk-dataset", name)]);

        DatasetClient::new(
            config,
            self.conn.ctx_read.clone(),
            self.conn.ctx_write.clone(),
        )
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
