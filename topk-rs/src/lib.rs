pub mod data;
pub mod error;
pub use error::Error;
pub mod expr;
pub mod query;

mod client;
pub use client::Client;
pub use client::ClientConfig;
pub use client::CollectionClient;
pub use client::CollectionsClient;

pub mod defaults {
    pub use crate::client::RETRY_BACKOFF_BASE;
    pub use crate::client::RETRY_BACKOFF_INIT;
    pub use crate::client::RETRY_BACKOFF_MAX;
    pub use crate::client::RETRY_MAX_RETRIES;
    pub use crate::client::RETRY_TIMEOUT;
}

pub use client::retry;
