use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(namespace = "data")]
#[derive(Debug, Clone)]
pub struct Vector(pub(crate) VectorUnion);

impl Vector {
    pub(crate) fn float(values: Vec<f32>) -> Self {
        Vector(VectorUnion::Float { values })
    }

    pub(crate) fn byte(values: Vec<u8>) -> Self {
        Vector(VectorUnion::Byte { values })
    }
}

#[napi(namespace = "data")]
impl Vector {
    #[napi]
    pub fn to_string(&self) -> String {
        format!("Vector({:?})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VectorUnion {
    Float { values: Vec<f32> },
    Byte { values: Vec<u8> },
}

impl Into<topk_rs::proto::v1::data::Vector> for Vector {
    fn into(self) -> topk_rs::proto::v1::data::Vector {
        match self.0 {
            VectorUnion::Float { values } => {
                topk_rs::proto::v1::data::Vector::f32(values.iter().map(|v| *v as f32).collect())
            }
            VectorUnion::Byte { values } => topk_rs::proto::v1::data::Vector::u8(values),
        }
    }
}

impl Into<topk_rs::proto::v1::data::Value> for Vector {
    fn into(self) -> topk_rs::proto::v1::data::Value {
        match self.0 {
            VectorUnion::Float { values } => topk_rs::proto::v1::data::Value::f32_vector(values),
            VectorUnion::Byte { values } => topk_rs::proto::v1::data::Value::u8_vector(values),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VectorData<T>(pub(crate) Vec<T>);

impl<T> Into<Vec<T>> for VectorData<T> {
    fn into(self) -> Vec<T> {
        self.0.into_iter().collect()
    }
}

impl<T> IntoIterator for VectorData<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> FromIterator<T> for VectorData<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        VectorData(iter.into_iter().collect())
    }
}

impl<T: ToNapiValue> ToNapiValue for VectorData<T> {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        Vec::<T>::to_napi_value(env, val.0)
    }
}

impl<T: FromNapiValue + ValidateNapiValue + std::fmt::Debug> FromNapiValue for VectorData<T> {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(array) = Vec::<T>::from_napi_value(env, value) {
            return Ok(VectorData(array));
        }

        Err(napi::Error::from_reason(
            "Invalid vector value, must be `number[]`",
        ))
    }
}
