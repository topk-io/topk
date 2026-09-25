mod client;
pub use client::Client;

mod transport;
pub use transport::Transport;

mod project_token;
pub use project_token::{ProjectAccessToken, ProjectToken};

pub mod proto {
    tonic::include_proto!("topk.management.v1");
}
