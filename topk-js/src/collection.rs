use std::sync::Arc;

use crate::{error::TopkError, query::Query};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data::ConsistencyLevel;

#[napi]
pub struct CollectionClient {
  collection: String,
  client: Arc<topk_rs::Client>,
}

#[napi]
impl CollectionClient {
  pub fn new(client: Arc<topk_rs::Client>, collection: String) -> Self {
    Self { client, collection }
  }

  #[napi]
  pub async fn query(&self, query: Query, lsn: Option<u32>) -> Result<String> {
    let docs = self
      .client
      .collection(&self.collection)
      .query(query.into(), lsn.map(|l| l as u64), None)
      .await
      .map_err(|e| match e {
        _ => panic!("failed to query collection: {:?}", e),
      })?;

    Ok(
      docs
        .into_iter()
        .map(|d| d.fields.into_iter().map(|(k, v)| (k, v.into())).collect())
        .collect(),
    )
  }
}
