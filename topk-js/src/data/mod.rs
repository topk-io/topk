mod collection;
pub use collection::{Collection, CollectionFieldSpec};

mod document;
pub use document::Document;

mod scalar;
pub use scalar::Scalar;

mod value;
pub use value::Value;

mod vector;
use vector::SparseVectorData;
pub use vector::{SparseVector, Vector};

use crate::data::vector::VectorData;
use napi_derive::napi;
use value::BytesData;

use napi::bindgen_prelude::FromNapiValue;

#[napi(namespace = "data")]
pub fn bytes(#[napi(ts_arg_type = "Array<number> | Buffer")] buffer: BytesData) -> Value {
    Value::Bytes(buffer.into())
}

#[napi(namespace = "data")]
pub fn f32_vector(#[napi(ts_arg_type = "Array<number>")] values: VectorData<f64>) -> Vector {
    Vector::float(values.into_iter().map(|v| v as f32).collect())
}

#[napi(namespace = "data")]
pub fn u8_vector(#[napi(ts_arg_type = "Array<number>")] values: VectorData<u8>) -> Vector {
    Vector::byte(values.into())
}

#[napi(namespace = "data")]
pub fn binary_vector(#[napi(ts_arg_type = "Array<number>")] values: VectorData<u8>) -> Vector {
    Vector::byte(values.into())
}

#[napi(namespace = "data")]
pub fn f32_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<f64>,
) -> SparseVector {
    SparseVector::float(vector.into_iter().map(|(i, v)| (i, v as f32)).collect())
}

#[napi(namespace = "data")]
pub fn u8_sparse_vector(
    #[napi(ts_arg_type = "Record<number, number>")] vector: SparseVectorData<u8>,
) -> SparseVector {
    SparseVector::byte(vector)
}

pub unsafe fn is_napi_integer(env: napi::sys::napi_env, napi_val: napi::sys::napi_value) -> bool {
    // Check if the number is an integer by comparing it with its integer part
    let num = f64::from_napi_value(env, napi_val).unwrap();
    num == (num as i64) as f64
}