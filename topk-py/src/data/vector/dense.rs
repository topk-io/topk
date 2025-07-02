use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum Vector {
    F32(Vec<f32>),
    U8(Vec<u8>),
}

impl From<Vector> for topk_rs::proto::v1::data::Vector {
    fn from(vector: Vector) -> Self {
        match vector {
            Vector::F32(values) => topk_rs::proto::v1::data::Vector::f32(values),
            Vector::U8(values) => topk_rs::proto::v1::data::Vector::u8(values),
        }
    }
}
