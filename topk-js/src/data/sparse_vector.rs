use napi::{bindgen_prelude::*, Error, Status};
use napi_derive::napi;
use std::{ffi::CString, iter::zip, ptr};

#[napi(namespace = "data")]
#[derive(Debug, Clone, PartialEq)]
pub enum SparseVectorUnion {
    Float {
        #[napi(ts_type = "Record<number, number>")]
        vector: SparseVectorF32,
    },
    Byte {
        #[napi(ts_type = "Record<number, number>")]
        vector: SparseVectorU8,
    },
}

#[napi(namespace = "data")]
#[derive(Debug, Clone)]
pub struct SparseVector(SparseVectorUnion);

#[napi]
impl SparseVector {
    pub fn new(values: SparseVectorUnion) -> Self {
        SparseVector(values)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparseVectorF32 {
    pub(crate) indices: Vec<u32>,
    pub(crate) values: Vec<f32>,
}

impl ToNapiValue for SparseVectorF32 {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let mut object = ptr::null_mut();
        check_status!(
            napi::sys::napi_create_object(env, &mut object),
            "Failed to create JavaScript object"
        )?;

        for (k, v) in zip(val.indices, val.values) {
            let key = CString::new(k.to_string())?;
            let value = f32::to_napi_value(env, v)?;

            check_status!(
                napi::sys::napi_set_named_property(env, object, key.as_ptr(), value),
                "Failed to set property"
            )?;
        }

        Ok(object)
    }
}

impl FromNapiValue for SparseVectorF32 {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let mut data_type: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut data_type);

        match data_type {
            napi::sys::ValueType::napi_object => {
                let object = Object::from_napi_value(env, value)?;

                let mut indices = Vec::new();
                let mut values = Vec::new();

                for key in Object::keys(&object)? {
                    let key = key.parse::<u32>().map_err(|_| {
                        Error::new(Status::InvalidArg, "Invalid key, must be u32".to_string())
                    })?;
                    indices.push(key);

                    let value =
                        object
                            .get_named_property::<f64>(&key.to_string())
                            .map_err(|_| {
                                Error::new(Status::InvalidArg, "Invalid value, must be f32")
                            })?;

                    values.push(value as f32);
                }

                Ok(SparseVectorF32 { indices, values })
            }
            _ => Err(Error::new(
                Status::InvalidArg,
                "Invalid vector. Expected an object with keys and values.".to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparseVectorU8 {
    pub(crate) indices: Vec<u32>,
    pub(crate) values: Vec<u8>,
}

impl ToNapiValue for SparseVectorU8 {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let mut object = ptr::null_mut();
        check_status!(
            napi::sys::napi_create_object(env, &mut object),
            "Failed to create JavaScript object"
        )?;

        for (k, v) in zip(val.indices, val.values) {
            let key = CString::new(k.to_string())?;
            let value = u8::to_napi_value(env, v)?;

            check_status!(
                napi::sys::napi_set_named_property(env, object, key.as_ptr(), value),
                "Failed to set property"
            )?;
        }

        Ok(object)
    }
}

impl FromNapiValue for SparseVectorU8 {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let mut data_type: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut data_type);

        match data_type {
            napi::sys::ValueType::napi_object => {
                let object = Object::from_napi_value(env, value)?;

                let mut indices = Vec::new();
                let mut values = Vec::new();

                for key in Object::keys(&object)? {
                    let key = key.parse::<u32>().map_err(|_| {
                        Error::new(Status::InvalidArg, "Invalid key, must be u32".to_string())
                    })?;
                    indices.push(key);

                    let value = object
                        .get_named_property::<u8>(&key.to_string())
                        .map_err(|_| Error::new(Status::InvalidArg, "Invalid value, must be u8"))?;

                    values.push(value);
                }

                Ok(SparseVectorU8 { indices, values })
            }
            _ => Err(Error::new(
                Status::InvalidArg,
                "Invalid vector. Expected an object with keys and values.".to_string(),
            )),
        }
    }
}

impl Into<topk_rs::data::SparseVector> for SparseVector {
    fn into(self) -> topk_rs::data::SparseVector {
        match self.0 {
            SparseVectorUnion::Float { vector } => topk_rs::data::SparseVector::F32 {
                indices: vector.indices,
                values: vector.values,
            },
            SparseVectorUnion::Byte { vector } => topk_rs::data::SparseVector::U8 {
                indices: vector.indices,
                values: vector.values,
            },
        }
    }
}

impl Into<topk_protos::v1::data::SparseVector> for SparseVector {
    fn into(self) -> topk_protos::v1::data::SparseVector {
        match self.0 {
            SparseVectorUnion::Float { vector } => {
                topk_protos::v1::data::SparseVector::f32(vector.indices, vector.values)
            }
            SparseVectorUnion::Byte { vector } => {
                topk_protos::v1::data::SparseVector::u8(vector.indices, vector.values)
            }
        }
    }
}

impl Into<topk_protos::v1::data::Value> for SparseVector {
    fn into(self) -> topk_protos::v1::data::Value {
        topk_protos::v1::data::Value {
            value: Some(topk_protos::v1::data::value::Value::SparseVector(
                self.into(),
            )),
        }
    }
}
