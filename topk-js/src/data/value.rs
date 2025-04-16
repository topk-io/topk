use std::ptr;

use napi::{bindgen_prelude::*, sys::napi_is_array};

use napi_derive::napi;

// #[napi]
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
    Binary(BinaryVector),
    Vector(Vector),
    Null,
}

#[napi]
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
}

#[napi]
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryVector {
    values: Vec<u8>,
}

#[napi]
impl BinaryVector {
    #[napi(getter)]
    pub fn get_values(&self) -> Vec<u8> {
        self.values.clone()
    }
}

impl FromNapiValue for BinaryVector {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        napi_val: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let env_env = Env::from_raw(env);

        // Check if it's a BinaryVector instance
        let is_binary_vector = {
            let env_value = Unknown::from_napi_value(env, napi_val)?;
            BinaryVector::instance_of(env_env, env_value)?
        };

        if is_binary_vector {
            let object = Object::from_napi_value(env, napi_val)?;
            // Get the values property from the JavaScript object
            let values: Option<Vec<u8>> = object.get("values")?;

            match values {
                Some(values) => Ok(BinaryVector { values }),
                None => Err(napi::Error::new(
                    napi::Status::GenericFailure,
                    "BinaryVector object missing 'values' property".to_string(),
                )),
            }
        } else {
            Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Value is not a binary vector",
            ))
        }
    }
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
pub fn binary_vector(values: Vec<u8>) -> BinaryVector {
    BinaryVector { values }
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
            Value::Binary(b) => topk_protos::v1::data::Value::binary(b.values),
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
            Some(topk_protos::v1::data::value::Value::Binary(b)) => {
                Value::Binary(BinaryVector { values: b })
            }
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
            napi::sys::ValueType::napi_number => {
                let f64_value = f64::from_napi_value(env, napi_val)?;
                if f64_value.fract() == 0.0 {
                    // integer
                    if f64_value >= 0.0 {
                        if f64_value < f64::from(u32::MAX) {
                            Ok(Value::U32(f64_value as u32))
                        } else if f64_value < u64::MAX as f64 {
                            Ok(Value::U64(f64_value as u64))
                        } else {
                            Ok(Value::I64(f64_value as i64))
                        }
                    } else {
                        // Negative integers
                        if f64_value >= f64::from(i32::MIN) {
                            Ok(Value::I32(f64_value as i32))
                        } else {
                            Ok(Value::I64(f64_value as i64))
                        }
                    }
                } else {
                    // Floating point
                    if f64_value.abs() < f32::MAX as f64 && (f64_value as f32) as f64 == f64_value {
                        Ok(Value::F32(f64_value as f32))
                    } else {
                        Ok(Value::F64(f64_value))
                    }
                }
            }
            napi::sys::ValueType::napi_boolean => {
                Ok(Value::Bool(bool::from_napi_value(env, napi_val)?))
            }
            napi::sys::ValueType::napi_object => {
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
                        let binary_vector = BinaryVector::from_napi_ref(env, napi_val);

                        match binary_vector {
                            Ok(vector) => {
                                return Ok(Value::Vector(Vector::Byte {
                                    values: vector.values.clone(),
                                }));
                            }
                            Err(_) => {}
                        }

                        let vector = Vector::from_napi_value(env, napi_val);

                        match vector {
                            Ok(Vector::Byte { values }) => {
                                return Ok(Value::Vector(Vector::Byte { values }));
                            }
                            Ok(Vector::Float { values }) => {
                                return Ok(Value::Vector(Vector::Float { values }));
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
            Value::Binary(b) => BinaryVector::to_napi_value(env, b),
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
            },
            Value::Null => Null::to_napi_value(env, Null),
        }
    }
}
