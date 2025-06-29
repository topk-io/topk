pub mod collection;
pub mod document;
pub mod scalar;
pub mod value;
pub mod vector;

use self::{
    value::Value,
    vector::{SparseVector, SparseVectorData, Vector},
};
use napi_derive::napi;

#[napi(namespace = "data")]
pub fn bytes(values: Vec<u8>) -> Value {
    Value::Bytes(values)
}

#[napi(namespace = "data")]
pub fn f32_vector(values: Vec<f64>) -> Vector {
    Vector::float(values.into_iter().map(|v| v as f32).collect())
}

#[napi(namespace = "data")]
pub fn u8_vector(values: Vec<u8>) -> Vector {
    Vector::byte(values)
}

#[napi(namespace = "data")]
pub fn binary_vector(values: Vec<u8>) -> Vector {
    Vector::byte(values)
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
