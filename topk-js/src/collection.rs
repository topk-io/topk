use crate::{
  document::{self, Document, Value},
  query::Query,
};
use napi::{bindgen_prelude::*, JsString, JsUnknown};
use napi_derive::napi;
use std::collections::HashMap;
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

  #[napi]
  pub async fn upsert(&self, docs: Vec<HashMap<String, Value>>) -> Result<i32> {
    // println!("{:?}", docs);

    let result = self
      .client
      .collection(&self.collection)
      .upsert(docs)
      .await
      .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))
      .map(|lsn| lsn as i32);

    // println!("{:?}", result);

    result
  }

  // #[napi]
  // pub fn testo(&self, docs: Vec<JsObject>) -> Result<()> {
  //   for doc in docs {
  //     let keys = Object::keys(&doc)?;
  //     println!("{:?}", keys);
  //     //   for key in keys {
  //     //     let value = doc.get::<Value>(&key)?;
  //     //     println!("{}: {:?}", key, value);
  //     //   }
  //   }
  //   Ok(())
  // }
}

// impl From<HashMap<String, document::Value>> for topk_protos::v1::data::Document {
//   fn from(map: HashMap<String, document::Value>) -> Self {
//     let mut doc = topk_protos::v1::data::Document::default();
//     for (key, value) in map {
//       doc.fields.insert(key, value.into());
//     }
//     doc
//   }
// }
