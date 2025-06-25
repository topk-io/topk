use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum Vector {
    F32(Vec<f32>),
    U8(Vec<u8>),
}

impl Into<topk_rs::data::Vector> for Vector {
    fn into(self) -> topk_rs::data::Vector {
        match self {
            Vector::F32(values) => topk_rs::data::Vector::F32(values),
            Vector::U8(values) => topk_rs::data::Vector::U8(values),
        }
    }
}
