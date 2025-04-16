use std::ptr;

use napi::{
    bindgen_prelude::*,
    sys::{napi_is_array, napi_is_buffer},
};

use napi_derive::napi;

use super::utils::is_napi_integer;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Bool(bool),
    F64(f64),
    U32(u32),
    U64(u64),
    I32(i32),
    I64(i64),
    F32(f32),
    Binary(Vec<u8>),
    Vector(Vector),
    Null,
}

#[napi(string_enum)]
#[derive(Debug, Clone, PartialEq)]
pub enum Vector {
    Float {
        #[napi(ts_type = "Array<number>")]
        values: Vec<f64>,
    },
    Byte {
        #[napi(ts_type = "Array<number>")]
        values: Vec<u8>,
    },
    Binary {
        #[napi(ts_type = "Array<number>")]
        values: Vec<u8>,
    },
}

#[napi]
pub fn f32_vector(values: Vec<f64>) -> Vector {
    Vector::Float { values }
}

#[napi]
pub fn u8_vector(values: Vec<u8>) -> Vector {
    Vector::Byte { values }
}

#[napi]
pub fn binary_vector(values: Vec<u8>) -> Vector {
    Vector::Binary { values }
}

#[napi]
pub fn binary(values: Vec<u8>) -> Value {
    Value::Binary(values)
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
                Vector::Float { values } => {
                    let vector = topk_protos::v1::data::Vector::float(
                        values.iter().map(|v| *v as f32).collect(),
                    );

                    topk_protos::v1::data::Value {
                        value: Some(topk_protos::v1::data::value::Value::Vector(vector)),
                    }
                }
                Vector::Byte { values } => {
                    let vector = topk_protos::v1::data::Vector::byte(values);

                    topk_protos::v1::data::Value {
                        value: Some(topk_protos::v1::data::value::Value::Vector(vector)),
                    }
                }
                Vector::Binary { values } => {
                    let vector = topk_protos::v1::data::Vector::byte(values);

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
            Some(topk_protos::v1::data::value::Value::U32(n)) => Value::I32(n.try_into().unwrap()),
            Some(topk_protos::v1::data::value::Value::U64(n)) => Value::U64(n.try_into().unwrap()),
            Some(topk_protos::v1::data::value::Value::I32(n)) => Value::I32(n),
            Some(topk_protos::v1::data::value::Value::I64(n)) => Value::I64(n),
            Some(topk_protos::v1::data::value::Value::F32(n)) => Value::F32(n),
            Some(topk_protos::v1::data::value::Value::Binary(b)) => Value::Binary(b),
            Some(topk_protos::v1::data::value::Value::Vector(v)) => match v.vector {
                Some(topk_protos::v1::data::vector::Vector::Float(float_vector)) => {
                    Value::Vector(Vector::Float {
                        values: float_vector.values.iter().map(|v| *v as f64).collect(),
                    })
                }
                Some(topk_protos::v1::data::vector::Vector::Byte(byte_vector)) => {
                    Value::Vector(Vector::Byte {
                        values: byte_vector.values,
                    })
                }
                None => unreachable!("Invalid vector proto"),
            },
            Some(topk_protos::v1::data::value::Value::Null(_)) => Value::Null,
            None => Value::Null,
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

                    return Ok(Value::Binary(values));
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
                            false => Ok(Value::Vector(Vector::Float {
                                values: vec_values_f64,
                            })),
                        }
                    }
                    false => {
                        let vector = Vector::from_napi_value(env, napi_val);

                        match vector {
                            Ok(Vector::Byte { values }) => {
                                return Ok(Value::Vector(Vector::Byte { values }));
                            }
                            Ok(Vector::Float { values }) => {
                                return Ok(Value::Vector(Vector::Float { values }));
                            }
                            Ok(Vector::Binary { values }) => {
                                return Ok(Value::Vector(Vector::Binary { values }));
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                }
            }
            napi::sys::ValueType::napi_null => Ok(Value::Null),
            napi::sys::ValueType::napi_undefined => Ok(Value::Null),
            _ => Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("Unsupported value type: {}", result),
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
            Value::Binary(b) => {
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
            Value::Vector(v) => match v {
                Vector::Float { values } => {
                    // Create a JavaScript array for the float vector
                    let mut js_array = ptr::null_mut();
                    check_status!(
                        sys::napi_create_array(env, &mut js_array),
                        "Failed to create JavaScript array"
                    )?;

                    // Add each float value to the array
                    for (i, &value) in values.iter().enumerate() {
                        let js_value = f64::to_napi_value(env, value)?;
                        check_status!(
                            sys::napi_set_element(env, js_array, i as u32, js_value),
                            "Failed to set array element"
                        )?;
                    }

                    Ok(js_array)
                }
                Vector::Byte { values } => {
                    let mut js_array = ptr::null_mut();

                    check_status!(
                        sys::napi_create_array(env, &mut js_array),
                        "Failed to create JavaScript array"
                    )?;

                    // Add each u8 value to the array
                    for (i, &value) in values.iter().enumerate() {
                        let js_value = u8::to_napi_value(env, value)?;
                        check_status!(
                            sys::napi_set_element(env, js_array, i as u32, js_value),
                            "Failed to set array element"
                        )?;
                    }

                    Ok(js_array)
                }
                Vector::Binary { values } => {
                    let mut js_array = ptr::null_mut();

                    check_status!(
                        sys::napi_create_array(env, &mut js_array),
                        "Failed to create JavaScript array"
                    )?;

                    // Add each u8 value to the array
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
