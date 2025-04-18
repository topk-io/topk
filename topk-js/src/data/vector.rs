use napi::{
    bindgen_prelude::{FromNapiRef, FromNapiValue},
    Error, Status,
};
use napi_derive::napi;

#[napi(namespace = "data")]
#[derive(Debug, Clone, PartialEq)]
pub enum VectorUnion {
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

#[napi(namespace = "data")]
#[derive(Debug, Clone)]
pub struct Vector(VectorUnion);

impl Vector {
    pub fn new(values: VectorUnion) -> Self {
        Vector(values)
    }

    pub fn value(&self) -> &VectorUnion {
        &self.0
    }
}

#[napi(namespace = "data")]
pub fn f32_vector(values: Vec<f64>) -> Vector {
    Vector(VectorUnion::Float { values })
}

#[napi(namespace = "data")]
pub fn u8_vector(values: Vec<u8>) -> Vector {
    Vector(VectorUnion::Byte { values })
}

#[napi(namespace = "data")]
pub fn binary_vector(values: Vec<u8>) -> Vector {
    Vector(VectorUnion::Binary { values })
}

impl FromNapiValue for Vector {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        unsafe {
            let mut is_array: bool = false;
            napi::sys::napi_is_array(env, value, &mut is_array);

            if is_array {
                let arr: Vec<f64> = Vec::from_napi_value(env, value)?;

                return Ok(Vector(VectorUnion::Float { values: arr }));
            } else {
                let vector = Vector::from_napi_ref(env, value);

                match vector {
                    Ok(vector) => Ok(Vector(vector.0.clone())),
                    Err(_) => Err(Error::new(
                        Status::InvalidArg,
                        "Invalid vector. Expected an array of numbers or a Vector object."
                            .to_string(),
                    )),
                }
            }
        }
    }
}

impl Into<topk_rs::data::Vector> for Vector {
    fn into(self) -> topk_rs::data::Vector {
        match self.0 {
            VectorUnion::Float { values } => {
                topk_rs::data::Vector::F32(values.iter().map(|v| *v as f32).collect())
            }
            VectorUnion::Byte { values } => topk_rs::data::Vector::U8(values),
            VectorUnion::Binary { values } => topk_rs::data::Vector::U8(values),
        }
    }
}

impl Into<topk_protos::v1::data::vector::Vector> for Vector {
    fn into(self) -> topk_protos::v1::data::vector::Vector {
        match self.0 {
            VectorUnion::Float { values } => {
                topk_protos::v1::data::vector::Vector::Float(topk_protos::v1::data::vector::Float {
                    values: values.iter().map(|v| *v as f32).collect(),
                })
            }
            VectorUnion::Byte { values } => {
                topk_protos::v1::data::vector::Vector::Byte(topk_protos::v1::data::vector::Byte {
                    values,
                })
            }
            VectorUnion::Binary { values } => {
                topk_protos::v1::data::vector::Vector::Byte(topk_protos::v1::data::vector::Byte {
                    values,
                })
            }
        }
    }
}

impl Into<topk_protos::v1::data::Vector> for Vector {
    fn into(self) -> topk_protos::v1::data::Vector {
        topk_protos::v1::data::Vector {
            vector: Some(self.into()),
        }
    }
}

impl Into<topk_protos::v1::data::Value> for Vector {
    fn into(self) -> topk_protos::v1::data::Value {
        topk_protos::v1::data::Value {
            value: Some(topk_protos::v1::data::value::Value::Vector(self.into())),
        }
    }
}
