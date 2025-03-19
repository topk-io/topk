#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

mod client;

use client::CollectionsClient;
use napi::bindgen_prelude::*;
use std::sync::Arc;

use topk_rs::{Client as RsClient, ClientConfig as RsClientConfig};

#[napi(object)]
pub struct ClientConfig {
  pub api_key: String,
  pub region: String,
  pub host: Option<String>,
  pub https: Option<bool>,
}

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
  pub fn collections(&self) -> CollectionsClient {
    CollectionsClient::new(self.client.clone())
  }
}