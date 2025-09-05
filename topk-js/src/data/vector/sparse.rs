use napi::{bindgen_prelude::*, Error, Status};
use napi_derive::napi;
use std::{ffi::CString, iter::zip, ptr};

pub(crate) const TYPE_ERROR: &str = "Invalid sparse vector, must be `Record<number, number>`";

#[napi(namespace = "data")]
#[derive(Debug, Clone)]
pub struct SparseVector(pub(crate) SparseVectorUnion);

impl SparseVector {
    pub(crate) fn float(vector: SparseVectorData<f32>) -> Self {
        SparseVector(SparseVectorUnion::Float { vector })
    }

    pub(crate) fn byte(vector: SparseVectorData<u8>) -> Self {
        SparseVector(SparseVectorUnion::Byte { vector })
    }
}

#[napi(namespace = "data")]
impl SparseVector {
    #[napi]
    pub fn to_string(&self) -> String {
        format!("SparseVector({:?})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SparseVectorUnion {
    Float { vector: SparseVectorData<f32> },
    Byte { vector: SparseVectorData<u8> },
}

impl Into<topk_rs::proto::v1::data::SparseVector> for SparseVector {
    fn into(self) -> topk_rs::proto::v1::data::SparseVector {
        match self.0 {
            SparseVectorUnion::Float { vector } => {
                topk_rs::proto::v1::data::SparseVector::f32(vector.indices, vector.values)
            }
            SparseVectorUnion::Byte { vector } => {
                topk_rs::proto::v1::data::SparseVector::u8(vector.indices, vector.values)
            }
        }
    }
}

impl Into<topk_rs::proto::v1::data::Value> for SparseVector {
    fn into(self) -> topk_rs::proto::v1::data::Value {
        match self.0 {
            SparseVectorUnion::Float { vector } => {
                topk_rs::proto::v1::data::Value::f32_sparse_vector(vector.indices, vector.values)
            }
            SparseVectorUnion::Byte { vector } => {
                topk_rs::proto::v1::data::Value::u8_sparse_vector(vector.indices, vector.values)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SparseVectorData<V> {
    pub(crate) indices: Vec<u32>,
    pub(crate) values: Vec<V>,
}

impl<T> IntoIterator for SparseVectorData<T> {
    type Item = (u32, T);
    type IntoIter = std::vec::IntoIter<(u32, T)>;
    fn into_iter(self) -> Self::IntoIter {
        self.indices
            .into_iter()
            .zip(self.values.into_iter())
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<T> FromIterator<(u32, T)> for SparseVectorData<T> {
    fn from_iter<I: IntoIterator<Item = (u32, T)>>(iter: I) -> Self {
        let mut indices = Vec::new();
        let mut values = Vec::new();

        for (index, value) in iter {
            indices.push(index);
            values.push(value);
        }

        SparseVectorData { indices, values }
    }
}

impl<T: ToNapiValue> ToNapiValue for SparseVectorData<T> {
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
            let value = T::to_napi_value(env, v)?;

            check_status!(
                napi::sys::napi_set_named_property(env, object, key.as_ptr(), value),
                "Failed to set property"
            )?;
        }

        Ok(object)
    }
}

impl<T: FromNapiValue + ValidateNapiValue + std::fmt::Debug> FromNapiValue for SparseVectorData<T> {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let mut is_array = false;
        check_status!(napi::sys::napi_is_array(env, value, &mut is_array))?;
        if is_array {
            return Err(Error::new(Status::InvalidArg, TYPE_ERROR));
        }

        let mut data_type: i32 = 0;
        check_status!(napi::sys::napi_typeof(env, value, &mut data_type))?;
        match data_type {
            napi::sys::ValueType::napi_object => {
                let object = Object::from_napi_value(env, value)?;
                
                // Check if this is the new format with 'indices' and 'values' keys
                if object.has_named_property("indices")? && object.has_named_property("values")? {
                    let indices: Vec<u32> = object.get_named_property("indices")
                        .map_err(|_| Error::new(Status::InvalidArg, "Invalid sparse vector, 'indices' must be an array of numbers"))?;
                    let values: Vec<T> = object.get_named_property("values")
                        .map_err(|_| Error::new(Status::InvalidArg, "Invalid sparse vector, 'values' must be an array"))?;
                    
                    if indices.len() != values.len() {
                        return Err(Error::new(Status::InvalidArg, 
                            "Invalid sparse vector, indices and values must have the same length"));
                    }
                    
                    // Validate that indices are sorted
                    for i in 1..indices.len() {
                        if indices[i] <= indices[i - 1] {
                            return Err(Error::new(Status::InvalidArg,
                                "Invalid sparse vector, indices must be sorted in ascending order and unique"));
                        }
                    }
                    
                    return Ok(SparseVectorData { indices, values });
                }
                
                // Otherwise treat as the old format {index: value}
                let mut indices = Vec::new();
                let mut values = Vec::new();

                for key in Object::keys(&object)? {
                    let key = key
                        .parse::<u32>()
                        .map_err(|_| Error::new(Status::InvalidArg, TYPE_ERROR))?;
                    indices.push(key);

                    let value = object
                        .get_named_property::<T>(&key.to_string())
                        .map_err(|_| Error::new(Status::InvalidArg, TYPE_ERROR))?;

                    values.push(value);
                }

                Ok(SparseVectorData { indices, values })
            }
            _ => Err(Error::new(Status::InvalidArg, TYPE_ERROR)),
        }
    }
}
