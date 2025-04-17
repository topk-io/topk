use napi::{
    bindgen_prelude::*,
    sys::{napi_get_value_bool, napi_typeof},
};

use crate::error::TopkError;

use super::utils::is_napi_integer;

#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    Bool(bool),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
}

impl Into<topk_rs::data::scalar::Scalar> for Scalar {
    fn into(self) -> topk_rs::data::scalar::Scalar {
        match self {
            Scalar::Bool(b) => topk_rs::data::scalar::Scalar::Bool(b),
            Scalar::U32(u) => topk_rs::data::scalar::Scalar::U32(u),
            Scalar::U64(u) => topk_rs::data::scalar::Scalar::U64(u),
            Scalar::I32(i) => topk_rs::data::scalar::Scalar::I32(i),
            Scalar::I64(i) => topk_rs::data::scalar::Scalar::I64(i),
            Scalar::F32(f) => topk_rs::data::scalar::Scalar::F32(f),
            Scalar::F64(f) => topk_rs::data::scalar::Scalar::F64(f),
            Scalar::String(s) => topk_rs::data::scalar::Scalar::String(s),
        }
    }
}

impl Into<topk_protos::v1::data::Value> for Scalar {
    fn into(self) -> topk_protos::v1::data::Value {
        match self {
            Scalar::Bool(b) => topk_protos::v1::data::Value::bool(b),
            Scalar::U32(u) => topk_protos::v1::data::Value::u32(u),
            Scalar::U64(u) => topk_protos::v1::data::Value::u64(u),
            Scalar::I32(i) => topk_protos::v1::data::Value::i32(i),
            Scalar::I64(i) => topk_protos::v1::data::Value::i64(i),
            Scalar::F32(f) => topk_protos::v1::data::Value::f32(f),
            Scalar::F64(f) => topk_protos::v1::data::Value::f64(f),
            Scalar::String(s) => topk_protos::v1::data::Value::string(s),
        }
    }
}

impl ToNapiValue for Scalar {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        match val {
            Scalar::Bool(b) => bool::to_napi_value(env, b),
            Scalar::U32(u) => u32::to_napi_value(env, u),
            Scalar::U64(u) => u32::to_napi_value(env, u as u32),
            Scalar::I32(i) => i32::to_napi_value(env, i),
            Scalar::I64(i) => i64::to_napi_value(env, i),
            Scalar::F32(f) => f32::to_napi_value(env, f),
            Scalar::F64(f) => f64::to_napi_value(env, f),
            Scalar::String(s) => String::to_napi_value(env, s),
        }
    }
}

impl FromNapiValue for Scalar {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let mut value_type = 0;

        napi_typeof(env, napi_val, &mut value_type);

        match value_type {
            napi::sys::ValueType::napi_boolean => {
                Ok(Scalar::Bool(bool::from_napi_value(env, napi_val)?))
            }
            napi::sys::ValueType::napi_number => {
                let is_integer = is_napi_integer(env, napi_val);

                match is_integer {
                    true => Ok(Scalar::I32(i32::from_napi_value(env, napi_val)?)),
                    false => Ok(Scalar::F64(f64::from_napi_value(env, napi_val)?)),
                }
            }
            napi::sys::ValueType::napi_string => {
                Ok(Scalar::String(String::from_napi_value(env, napi_val)?))
            }
            _ => Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Invalid scalar type",
            )),
        }
    }
}
