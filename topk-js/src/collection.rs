use crate::{document::Document, query::Query};
use napi::{bindgen_prelude::*, CallContext};
use napi_derive::napi;
use std::sync::Arc;

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
  pub async fn query(&self, query: Query, lsn: Option<u32>) -> Result<Vec<Document>> {
    let docs = self
      .client
      .collection(&self.collection)
      .query(query.into(), lsn.map(|l| l as u64), None)
      .await
      .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;

    Ok(docs.into_iter().map(|d| d.into()).collect())
  }
}
