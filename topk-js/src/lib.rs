#![deny(clippy::all)]

mod binary_expr;
mod client;
mod collection;
mod collections;
mod document;
mod error;
mod filter_expr;
mod function_expr;
mod logical_expr;
mod napi_box;
mod query;
mod select_expr;
mod text_expr;
mod unary_expr;
use crate::client::ClientConfig;
use napi_derive::napi;
use std::sync::Arc;
use topk_rs::{Client as RsClient, ClientConfig as RsClientConfig};

#[napi]
pub struct Client {
  client: Arc<RsClient>,
}

#[napi]
impl Client {
  #[napi(constructor)]
  pub fn new(config: ClientConfig) -> Self {
    let mut rs_config = RsClientConfig::new(config.api_key, config.region);

    if let Some(host_value) = config.host {
      rs_config = rs_config.with_host(host_value);
    }

    if let Some(https_value) = config.https {
      rs_config = rs_config.with_https(https_value);
    }

    let client = Arc::new(RsClient::new(rs_config));

    Self { client }
  }

  #[napi]
  pub fn collections(&self) -> collections::CollectionsClient {
    collections::CollectionsClient::new(self.client.clone())
  }

  #[napi]
  pub fn collection(&self, name: String) -> collection::CollectionClient {
    collection::CollectionClient::new(self.client.clone(), name)
  }
}
