use std::ptr;

use napi::{
    bindgen_prelude::*,
    sys::{napi_is_array, napi_is_buffer},
};

use napi_derive::napi;

use super::{
    utils::{get_napi_value_type, is_napi_integer},
    vector::{Vector, VectorUnion},
};

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Bool(bool),
    F64(f64),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    Bytes(Vec<u8>),
    Vector(Vector),
    Null,
}

#[napi(namespace = "data")]
pub fn bytes(values: Vec<u8>) -> Value {
    Value::Bytes(values)
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
            Value::Bytes(b) => topk_protos::v1::data::Value::bytes(b),
            Value::Vector(v) => v.into(),
            Value::Null => topk_protos::v1::data::Value::null(),
        }
    }
}

impl From<topk_protos::v1::data::Value> for Value {
    fn from(value: topk_protos::v1::data::Value) -> Self {
        match value.value {
            Some(topk_protos::v1::data::value::Value::String(s)) => Value::String(s),
            Some(topk_protos::v1::data::value::Value::F64(n)) => Value::F64(n),
            Some(topk_protos::v1::data::value::Value::Bool(b)) => Value::Bool(b),
            Some(topk_protos::v1::data::value::Value::U32(n)) => {
                Value::I32(n.try_into().expect("U32 is lossy"))
            }
            Some(topk_protos::v1::data::value::Value::U64(n)) => {
                Value::U64(n.try_into().expect("U64 is lossy"))
            }
            Some(topk_protos::v1::data::value::Value::I32(n)) => Value::I32(n),
            Some(topk_protos::v1::data::value::Value::I64(n)) => Value::I64(n),
            Some(topk_protos::v1::data::value::Value::F32(n)) => Value::F32(n),
            Some(topk_protos::v1::data::value::Value::Binary(b)) => Value::Bytes(b),
            Some(topk_protos::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_protos::v1::data::vector::Vector::Float(float_vector)) => {
                    Value::Vector(Vector::new(VectorUnion::Float {
                        values: float_vector.values.iter().map(|v| *v as f64).collect(),
                    }))
                }
                Some(topk_protos::v1::data::vector::Vector::Byte(byte_vector)) => {
                    Value::Vector(Vector::new(VectorUnion::Byte {
                        values: byte_vector.values,
                    }))
                }
                None => unreachable!("Invalid vector proto"),
            },
            Some(topk_protos::v1::data::value::Value::Null(_)) => Value::Null,
            None => unreachable!("Invalid proto"),
        }
    }
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
            napi::sys::ValueType::napi_number => match is_napi_integer(env, napi_val) {
                true => Ok(Value::I32(i32::from_napi_value(env, napi_val)?)),
                false => Ok(Value::F64(f64::from_napi_value(env, napi_val)?)),
            },
            napi::sys::ValueType::napi_boolean => {
                Ok(Value::Bool(bool::from_napi_value(env, napi_val)?))
            }
            napi::sys::ValueType::napi_object => {
                let mut is_buffer: bool = false;
                napi_is_buffer(env, napi_val, &mut is_buffer);

                if is_buffer {
                    let buffer = Buffer::from_napi_value(env, napi_val)?;
                    let values: Vec<u8> = buffer.into();

                    return Ok(Value::Bytes(values));
                }

                let mut is_array: bool = false;
                napi_is_array(env, napi_val, &mut is_array);

                match is_array {
                    true => {
                        let mut vec_values_f64 = Vec::<f64>::new();
                        let mut length_result: u32 = 0;
                        napi::sys::napi_get_array_length(env, napi_val, &mut length_result);

                        for i in 0..length_result {
                            let mut value_result = ptr::null_mut();
                            napi::sys::napi_get_element(env, napi_val, i, &mut value_result);

                            let mut value_type_result: i32 = 0;
                            napi::sys::napi_typeof(env, value_result, &mut value_type_result);

                            match value_type_result {
                                napi::sys::ValueType::napi_number => {
                                    vec_values_f64.push(f64::from_napi_value(env, value_result)?);
                                }
                                _type => {
                                    return Err(napi::Error::new(
                                        napi::Status::GenericFailure,
                                        format!(
                                            "Vector elements must be of type `number`, got: `{:?}`",
                                            _type
                                        ),
                                    ));
                                }
                            }
                        }

                        match vec_values_f64.is_empty() {
                            true => Err(napi::Error::new(
                                napi::Status::GenericFailure,
                                "Vector is empty: {}",
                            )),
                            false => Ok(Value::Vector(Vector::new(VectorUnion::Float {
                                values: vec_values_f64,
                            }))),
                        }
                    }
                    false => {
                        let vector = Vector::from_napi_ref(env, napi_val);

                        match vector {
                            Ok(vector) => Ok(Value::Vector(vector.clone())),
                            Err(e) => Err(e),
                        }
                    }
                }
            }
            napi::sys::ValueType::napi_null => Ok(Value::Null),
            napi::sys::ValueType::napi_undefined => Ok(Value::Null),
            _ => Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("Unsupported value type: {}", get_napi_value_type(result)),
            )),
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
            Value::U64(n) => i64::to_napi_value(env, n.try_into().unwrap()),
            Value::I32(n) => i32::to_napi_value(env, n),
            Value::I64(n) => i64::to_napi_value(env, n),
            Value::F32(n) => f32::to_napi_value(env, n),
            Value::Bytes(b) => {
                let mut js_array = ptr::null_mut();
                check_status!(
                    sys::napi_create_array(env, &mut js_array),
                    "Failed to create JavaScript array"
                )?;

                for (i, &value) in b.iter().enumerate() {
                    let js_value = u8::to_napi_value(env, value)?;
                    check_status!(
                        sys::napi_set_element(env, js_array, i as u32, js_value),
                        "Failed to set array element"
                    )?;
                }

                Ok(js_array)
            }
            Value::Vector(v) => match v.value() {
                VectorUnion::Float { values } => {
                    let mut js_array = ptr::null_mut();
                    check_status!(
                        sys::napi_create_array(env, &mut js_array),
                        "Failed to create JavaScript array"
                    )?;

                    for (i, &value) in values.iter().enumerate() {
                        let js_value = f64::to_napi_value(env, value)?;
                        check_status!(
                            sys::napi_set_element(env, js_array, i as u32, js_value),
                            "Failed to set array element"
                        )?;
                    }

                    Ok(js_array)
                }
                VectorUnion::Byte { values } | VectorUnion::Binary { values } => {
                    let mut js_array = ptr::null_mut();
                    check_status!(
                        sys::napi_create_array(env, &mut js_array),
                        "Failed to create JavaScript array"
                    )?;

                    for (i, &value) in values.iter().enumerate() {
                        let js_value = u8::to_napi_value(env, value)?;
                        check_status!(
                            sys::napi_set_element(env, js_array, i as u32, js_value),
                            "Failed to set array element"
                        )?;
                    }

                    Ok(js_array)
                }
            },
            Value::Null => Null::to_napi_value(env, Null),
        }
    }
}
