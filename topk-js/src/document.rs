use napi_derive::napi;
use std::collections::HashMap;

#[napi]
pub enum Value {
  String(String),
  F64(f64),
}

#[napi]
pub struct Document {
  fields: HashMap<String, Value>,
}

impl From<Document> for topk_protos::v1::data::Document {
  fn from(doc: Document) -> Self {
    topk_protos::v1::data::Document {
      fields: doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
    }
  }
}

impl From<topk_protos::v1::data::Document> for Document {
  fn from(doc: topk_protos::v1::data::Document) -> Self {
    Document {
      fields: doc.fields.into_iter().map(|(k, v)| (k, v.into())).collect(),
    }
  }
}

impl From<Value> for topk_protos::v1::data::Value {
  fn from(value: Value) -> Self {
    match value {
      Value::String(s) => topk_protos::v1::data::Value::string(s),
      Value::F64(n) => topk_protos::v1::data::Value::f64(n),
    }
  }
}

impl From<topk_protos::v1::data::Value> for Value {
  fn from(value: topk_protos::v1::data::Value) -> Self {
    match value.value {
      Some(topk_protos::v1::data::value::Value::String(s)) => Value::String(s),
      Some(topk_protos::v1::data::value::Value::F64(n)) => Value::F64(n),
      t => panic!("unsupported value type: {:?}", t),
    }
  }
}
