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
