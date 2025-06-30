use super::vector::{SparseVector, Vector};
use crate::data::vector::{SparseVectorData, SparseVectorUnion, VectorData, VectorUnion};
use napi::bindgen_prelude::*;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    U32(u32),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Vector(Vector),
    SparseVector(SparseVector),
    Bytes(Vec<u8>),
}

impl From<Value> for topk_rs::proto::v1::data::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::Null => topk_rs::proto::v1::data::Value::null(),
            Value::Bool(b) => topk_rs::proto::v1::data::Value::bool(b),
            Value::String(s) => topk_rs::proto::v1::data::Value::string(s),
            Value::U32(n) => topk_rs::proto::v1::data::Value::u32(n),
            Value::I32(n) => topk_rs::proto::v1::data::Value::i32(n),
            Value::I64(n) => topk_rs::proto::v1::data::Value::i64(n),
            Value::F32(n) => topk_rs::proto::v1::data::Value::f32(n),
            Value::F64(n) => topk_rs::proto::v1::data::Value::f64(n),
            Value::Bytes(b) => topk_rs::proto::v1::data::Value::bytes(b),
            Value::Vector(v) => topk_rs::proto::v1::data::Value::vector(v),
            Value::SparseVector(v) => topk_rs::proto::v1::data::Value::sparse_vector(v),
        }
    }
}

impl From<topk_rs::proto::v1::data::Value> for Value {
    fn from(value: topk_rs::proto::v1::data::Value) -> Self {
        match value.value {
            // Null
            Some(topk_rs::proto::v1::data::value::Value::Null(_)) => Value::Null,
            // Bool
            Some(topk_rs::proto::v1::data::value::Value::Bool(b)) => Value::Bool(b),
            // String
            Some(topk_rs::proto::v1::data::value::Value::String(s)) => Value::String(s),
            // Numbers
            Some(topk_rs::proto::v1::data::value::Value::F64(n)) => Value::F64(n),
            Some(topk_rs::proto::v1::data::value::Value::F32(n)) => Value::F64(n as f64),
            Some(topk_rs::proto::v1::data::value::Value::U32(n)) => {
                Value::U32(n.try_into().expect("U32 is lossy"))
            }
            Some(topk_rs::proto::v1::data::value::Value::U64(n)) => {
                Value::U32(n.try_into().expect("U32 is lossy"))
            }
            Some(topk_rs::proto::v1::data::value::Value::I32(n)) => Value::I32(n),
            Some(topk_rs::proto::v1::data::value::Value::I64(n)) => Value::I64(n),
            // Bytes
            Some(topk_rs::proto::v1::data::value::Value::Binary(b)) => Value::Bytes(b),
            // Vectors
            Some(topk_rs::proto::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_rs::proto::v1::data::vector::Vector::Float(float_vector)) => {
                    Value::Vector(Vector::float(float_vector.values))
                }
                Some(topk_rs::proto::v1::data::vector::Vector::Byte(byte_vector)) => {
                    Value::Vector(Vector::byte(byte_vector.values))
                }
                None => unreachable!("Invalid vector proto"),
            },
            // Sparse vectors
            Some(topk_rs::proto::v1::data::value::Value::SparseVector(sparse_vector)) => {
                Value::SparseVector(match sparse_vector.values {
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::F32(values)) => {
                        SparseVector::float(SparseVectorData::<f32> {
                            indices: sparse_vector.indices,
                            values: values.values,
                        })
                    }
                    Some(topk_rs::proto::v1::data::sparse_vector::Values::U8(values)) => {
                        SparseVector::byte(SparseVectorData::<u8> {
                            indices: sparse_vector.indices,
                            values: values.values,
                        })
                    }
                    None => unreachable!("Invalid sparse vector proto"),
                })
            }
            None => unreachable!("Invalid value proto"),
        }
    }
}

impl FromNapiValue for Value {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(sparse_vector) = crate::try_cast_ref!(env, value, SparseVector) {
            return Ok(Value::SparseVector(sparse_vector.clone()));
        }

        if let Ok(vector) = crate::try_cast_ref!(env, value, Vector) {
            return Ok(Value::Vector(vector.clone()));
        }

        let mut result: i32 = 0;
        check_status!(napi::sys::napi_typeof(env, value, &mut result))?;
        match result {
            napi::sys::ValueType::napi_undefined => Ok(Value::Null),
            napi::sys::ValueType::napi_null => Ok(Value::Null),
            napi::sys::ValueType::napi_string => {
                Ok(Value::String(String::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_number => match is_napi_integer(env, value) {
                true => Ok(Value::I64(i64::from_napi_value(env, value)?)),
                false => Ok(Value::F64(f64::from_napi_value(env, value)?)),
            },
            napi::sys::ValueType::napi_boolean => {
                Ok(Value::Bool(bool::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_object => {
                // Vectors
                if let Ok(vector) = VectorData::<f64>::from_napi_value(env, value) {
                    return Ok(Value::Vector(Vector::float(
                        vector.into_iter().map(|v| v as f32).collect(),
                    )));
                }

                // Sparse vectors (all "naked" sparse vectors are interpreted as f32)
                if let Ok(sparse_vector) = SparseVectorData::<f64>::from_napi_value(env, value) {
                    return Ok(Value::SparseVector(SparseVector::float(
                        sparse_vector
                            .into_iter()
                            .map(|(i, v)| (i, v as f32))
                            .collect(),
                    )));
                }

                // Bytes/buffers
                if let Ok(buffer) = Buffer::from_napi_value(env, value) {
                    return Ok(Value::Bytes(buffer.to_vec()));
                }

                return Err(napi::Error::from_reason(
                    "Unsupported object type".to_string(),
                ));
            }
            _ => Err(napi::Error::from_reason(format!(
                "Unsupported napi value type: {:?}",
                value
            ))),
        }
    }
}

impl ToNapiValue for Value {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> Result<sys::napi_value> {
        match val {
            Value::String(s) => String::to_napi_value(env, s),
            Value::F64(n) => f64::to_napi_value(env, n),
            Value::Bool(b) => bool::to_napi_value(env, b),
            Value::U32(n) => u32::to_napi_value(env, n),
            Value::I32(n) => i32::to_napi_value(env, n),
            Value::I64(n) => i64::to_napi_value(env, n),
            Value::F32(n) => f32::to_napi_value(env, n),
            Value::Bytes(b) => Buffer::to_napi_value(env, b.into()),
            Value::Vector(v) => match v.0 {
                VectorUnion::Float { values } => Vec::<f32>::to_napi_value(env, values),
                VectorUnion::Byte { values } => Vec::<u8>::to_napi_value(env, values),
            },
            Value::SparseVector(v) => match v.0 {
                SparseVectorUnion::Float { vector } => {
                    SparseVectorData::<f32>::to_napi_value(env, vector)
                }
                SparseVectorUnion::Byte { vector } => {
                    SparseVectorData::<u8>::to_napi_value(env, vector)
                }
            },
            Value::Null => Null::to_napi_value(env, Null),
        }
    }
}

unsafe fn is_napi_integer(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> bool {
    // Check if the number is an integer by comparing it with its integer part
    let num = f64::from_napi_value(env, napi_val).unwrap();
    num == (num as i64) as f64
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BytesData(pub(crate) Vec<u8>);

impl Into<Vec<u8>> for BytesData {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

impl FromIterator<u8> for BytesData {
    fn from_iter<I: IntoIterator<Item = u8>>(iter: I) -> Self {
        BytesData(iter.into_iter().collect())
    }
}

impl ToNapiValue for BytesData {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        Vec::<u8>::to_napi_value(env, val.0)
    }
}

impl FromNapiValue for BytesData {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(array) = Vec::<u8>::from_napi_value(env, value) {
            return Ok(BytesData(array));
        }

        if let Ok(buffer) = Buffer::from_napi_value(env, value) {
            return Ok(BytesData(buffer.to_vec()));
        }

        Err(napi::Error::from_reason(
            "Invalid bytes value, must be `number[]` or `Buffer`",
        ))
    }
}
