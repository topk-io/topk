mod client;
pub use client::Client;

mod transport;
pub use transport::Transport;

mod project_tokens;
pub use project_tokens::{ProjectTokenInterceptor, ProjectTokens};

pub mod proto {
    tonic::include_proto!("topk.management.v1");
}
