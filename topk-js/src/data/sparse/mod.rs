use napi::bindgen_prelude::*;
use napi_derive::napi;

pub mod f32;
pub mod u8;

#[napi(namespace = "data")]
#[derive(Debug, Clone)]
pub struct SparseVector(SparseVectorUnion);

#[napi(namespace = "data")]
#[derive(Debug, Clone, PartialEq)]
pub enum SparseVectorUnion {
    Float {
        #[napi(ts_type = "Record<number, number>")]
        vector: f32::SparseVectorF32,
    },
    Byte {
        #[napi(ts_type = "Record<number, number>")]
        vector: u8::SparseVectorU8,
    },
}

#[napi]
impl SparseVector {
    pub fn new(values: SparseVectorUnion) -> Self {
        SparseVector(values)
    }

    pub fn value(&self) -> &SparseVectorUnion {
        &self.0
    }
}

impl FromNapiValue for SparseVector {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        todo!()
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
        topk_rs::proto::v1::data::Value {
            value: Some(topk_rs::proto::v1::data::value::Value::SparseVector(
                self.into(),
            )),
        }
    }
}
