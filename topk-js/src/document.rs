use napi::{
  bindgen_prelude::{FromNapiValue, ToNapiValue},
  sys,
};
use napi_derive::napi;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Value {
  String(String),
  F64(f64),
}

impl FromNapiValue for Value {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    napi_val: napi::sys::napi_value,
  ) -> napi::Result<Self> {
    let mut result: i32 = 0;
    napi::sys::napi_typeof(env, napi_val, &mut result);

    match result {
      napi::sys::ValueType::napi_string => {
        Ok(Value::String(String::from_napi_value(env, napi_val)?))
      }
      napi::sys::ValueType::napi_number => Ok(Value::F64(f64::from_napi_value(env, napi_val)?)),
      _ => panic!("unsupported value type: {:?}", result),
    }
  }
}

impl ToNapiValue for Value {
  unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value, napi::Error> {
    match val {
      Value::String(s) => String::to_napi_value(env, s),
      Value::F64(n) => f64::to_napi_value(env, n),
    }
  }
}

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

pub struct DocumentWrapper(pub topk_protos::v1::data::Document);

impl From<topk_protos::v1::data::Document> for DocumentWrapper {
  fn from(doc: topk_protos::v1::data::Document) -> Self {
    Self(doc)
  }
}

impl From<DocumentWrapper> for HashMap<String, Value> {
  fn from(wrapper: DocumentWrapper) -> Self {
    wrapper
      .0
      .fields
      .into_iter()
      .map(|(k, v)| (k, v.into()))
      .collect()
  }
}
