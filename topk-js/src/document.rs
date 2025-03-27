use napi::{
  bindgen_prelude::{check_status, FromNapiValue, Null, ToNapiValue},
  sys::{self, TypedarrayType},
};
use napi_derive::napi;
use std::{collections::HashMap, ptr};

#[derive(Debug, Clone)]
pub enum Value {
  String(String),
  F64(f64),
  Bool(bool),
  U32(u32),
  U64(u64),
  I32(i32),
  I64(i64),
  F32(f32),
  Binary(Vec<u8>),
  Vector(Vector),
  Null,
}

#[derive(Debug, Clone)]
pub enum Vector {
  Float(Vec<f32>),
  Byte(Vec<u8>),
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
      Value::Bool(b) => bool::to_napi_value(env, b),
      Value::U32(n) => u32::to_napi_value(env, n),
      Value::U64(n) => u32::to_napi_value(env, n as u32),
      Value::I32(n) => i32::to_napi_value(env, n),
      Value::I64(n) => i64::to_napi_value(env, n),
      Value::F32(n) => f32::to_napi_value(env, n),
      Value::Binary(b) => {
        todo!()
      }
      Value::Vector(v) => match v {
        Vector::Float(values) => {
          // Create a JavaScript array for the float vector
          let mut js_array = ptr::null_mut();
          check_status!(
            sys::napi_create_array(env, &mut js_array),
            "Failed to create JavaScript array"
          )?;

          // Add each float value to the array
          for (i, &value) in values.iter().enumerate() {
            let js_value = f32::to_napi_value(env, value)?;
            check_status!(
              sys::napi_set_element(env, js_array, i as u32, js_value),
              "Failed to set array element"
            )?;
          }

          Ok(js_array)
        }
        Vector::Byte(values) => {
          // Create a Uint8Array for byte vector
          let mut arraybuffer = ptr::null_mut();
          let length = values.len();

          check_status!(
            sys::napi_create_arraybuffer(env, length, &mut arraybuffer, ptr::null_mut()),
            "Failed to create ArrayBuffer"
          )?;

          let mut typed_array = ptr::null_mut();
          check_status!(
            sys::napi_create_typedarray(
              env,
              TypedarrayType::uint8_array,
              length,
              arraybuffer as *mut sys::napi_value__,
              0,
              &mut typed_array
            ),
            "Failed to create Uint8Array"
          )?;

          // Copy the bytes into the array buffer
          let mut data_ptr = ptr::null_mut();
          check_status!(
            sys::napi_get_arraybuffer_info(
              env,
              arraybuffer as *mut sys::napi_value__,
              &mut data_ptr,
              ptr::null_mut()
            ),
            "Failed to get ArrayBuffer info"
          )?;

          std::ptr::copy_nonoverlapping(values.as_ptr(), data_ptr as *mut u8, length);

          Ok(typed_array)
        }
      },
      Value::Null => Null::to_napi_value(env, Null),
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
      Value::Bool(b) => topk_protos::v1::data::Value::bool(b),
      Value::U32(n) => topk_protos::v1::data::Value::u32(n),
      Value::U64(n) => topk_protos::v1::data::Value::u64(n),
      Value::I32(n) => topk_protos::v1::data::Value::i32(n),
      Value::I64(n) => topk_protos::v1::data::Value::i64(n),
      Value::F32(n) => topk_protos::v1::data::Value::f32(n),
      Value::Binary(b) => topk_protos::v1::data::Value::binary(b),
      Value::Vector(v) => match v {
        Vector::Float(values) => {
          let float_vector = topk_protos::v1::data::vector::Float { values };
          let vector = topk_protos::v1::data::Vector {
            vector: Some(topk_protos::v1::data::vector::Vector::Float(float_vector)),
          };
          topk_protos::v1::data::Value {
            value: Some(topk_protos::v1::data::value::Value::Vector(vector)),
          }
        }
        Vector::Byte(values) => {
          let byte_vector = topk_protos::v1::data::vector::Byte { values };
          let vector = topk_protos::v1::data::Vector {
            vector: Some(topk_protos::v1::data::vector::Vector::Byte(byte_vector)),
          };
          topk_protos::v1::data::Value {
            value: Some(topk_protos::v1::data::value::Value::Vector(vector)),
          }
        }
      },
      Value::Null => topk_protos::v1::data::Value::null(),
    }
  }
}

impl From<topk_protos::v1::data::Value> for Value {
  fn from(value: topk_protos::v1::data::Value) -> Self {
    match value.value {
      Some(topk_protos::v1::data::value::Value::String(s)) => Value::String(s),
      Some(topk_protos::v1::data::value::Value::F64(n)) => Value::F64(n),
      Some(topk_protos::v1::data::value::Value::Bool(b)) => Value::String(b.to_string()),
      Some(topk_protos::v1::data::value::Value::U32(n)) => Value::F64(n as f64),
      Some(topk_protos::v1::data::value::Value::U64(n)) => Value::F64(n as f64),
      Some(topk_protos::v1::data::value::Value::I32(n)) => Value::F64(n as f64),
      Some(topk_protos::v1::data::value::Value::I64(n)) => Value::F64(n as f64),
      Some(topk_protos::v1::data::value::Value::F32(n)) => Value::F64(n as f64),
      Some(topk_protos::v1::data::value::Value::Binary(b)) => Value::String(format!("{:?}", b)),
      Some(topk_protos::v1::data::value::Value::Vector(v)) => match v.vector {
        Some(topk_protos::v1::data::vector::Vector::Float(float_vector)) => {
          Value::Vector(Vector::Float(float_vector.values))
        }
        Some(topk_protos::v1::data::vector::Vector::Byte(byte_vector)) => {
          Value::Vector(Vector::Byte(byte_vector.values))
        }
        None => Value::Null,
      },
      Some(topk_protos::v1::data::value::Value::Null(_)) => Value::Null,
      None => Value::String("undefined".to_string()),
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
