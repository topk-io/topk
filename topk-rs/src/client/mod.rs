use crate::create_client;
use std::sync::Arc;
use tokio::sync::OnceCell;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;
use topk_protos::v1::control::collection_service_client::CollectionServiceClient;
use topk_protos::v1::data::query_service_client::QueryServiceClient;
use topk_protos::v1::data::write_service_client::WriteServiceClient;

mod collections;
pub use collections::CollectionsClient;

mod collection;
pub use collection::CollectionClient;

mod config;
pub use config::ClientConfig;

pub mod retry;

mod interceptor;
pub use interceptor::AppendHeadersInterceptor;

// (client) max message size for all requests
pub const MAX_DECODING_MESSAGE_SIZE: usize = 512 * 1024 * 1024; // 512MB
pub const MAX_ENCODING_MESSAGE_SIZE: usize = 512 * 1024 * 1024; // 512MB

// request config
pub const TIMEOUT: u64 = 60_000; // 1 minute
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
    config: Arc<ClientConfig>,

    // Channels
    channel: Arc<OnceCell<Channel>>,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config: Arc::new(config),
            channel: Arc::new(OnceCell::new()),
        }
    }

    #[cfg(feature = "in_memory")]
    pub fn new_in_memory(config: ClientConfig, channel: Channel) -> Self {
        Self {
            config: Arc::new(config),
            channel: Arc::new(OnceCell::new_with(Some(channel))),
        }
    }

    // Collection operations (Control plane)
    pub fn collections(&self) -> CollectionsClient {
        CollectionsClient::new(&self.config, &self.channel)
    }

    // Document operations (Data plane)
    pub fn collection(&self, name: impl Into<String>) -> CollectionClient {
        CollectionClient::new(self.config.clone(), self.channel.clone(), name.into())
    }
}

// Macro for instantiating and connecting a client
#[macro_export]
macro_rules! create_client {
    ($client:ident, $channel:expr, $endpoint:expr, $headers:expr) => {
        async {
            use std::str::FromStr;

            let channel = $channel
                .get_or_try_init(|| async {
                    Ok(tonic::transport::Endpoint::from_str($endpoint)?
                        .tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())?
                        // Do not close idle connections so they can be reused
                        .keep_alive_while_idle(true)
                        // Set max header list size to 64KB
                        .http2_max_header_list_size(crate::client::MAX_HEADER_LIST_SIZE)
                        // Request timeout
                        .timeout(std::time::Duration::from_secs(crate::client::TIMEOUT))
                        // Connect
                        .connect()
                        .await?)
                })
                .await;

            match channel {
                Ok(channel) => {
                    let client = $client::with_interceptor(
                        channel.clone(),
                        crate::client::AppendHeadersInterceptor::new($headers),
                    )
                    .max_decoding_message_size(crate::client::MAX_DECODING_MESSAGE_SIZE)
                    .max_encoding_message_size(crate::client::MAX_ENCODING_MESSAGE_SIZE);

                    Ok(client)
                }
                // If channel fails to connect, return the error immediately
                Err(e) => Err(e),
            }
        }
    };
}

// Clients
async fn create_query_client<'a>(
    config: &'a ClientConfig,
    collection: &'a str,
    channel: &'a OnceCell<Channel>,
) -> Result<QueryServiceClient<InterceptedService<Channel, AppendHeadersInterceptor>>, super::Error>
{
    let config = config
        .clone()
        .with_headers([("x-topk-collection", collection.to_string())]);

    create_client!(
        QueryServiceClient,
        channel,
        &config.endpoint(),
        config.headers()
    )
    .await
}

async fn create_write_client<'a>(
    config: &'a ClientConfig,
    collection: &'a str,
    channel: &'a OnceCell<Channel>,
) -> Result<WriteServiceClient<InterceptedService<Channel, AppendHeadersInterceptor>>, super::Error>
{
    let config = config
        .clone()
        .with_headers([("x-topk-collection", collection.to_string())]);

    create_client!(
        WriteServiceClient,
        channel,
        &config.endpoint(),
        config.headers()
    )
    .await
}

async fn create_collection_client<'a>(
    config: &'a ClientConfig,
    channel: &'a OnceCell<Channel>,
) -> Result<
    CollectionServiceClient<InterceptedService<Channel, AppendHeadersInterceptor>>,
    super::Error,
> {
    create_client!(
        CollectionServiceClient,
        channel,
        &config.endpoint(),
        config.headers()
    )
    .await
}
