use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum Matrix {
    F32 { dimension: u32, values: Vec<f32> },
    U8 { dimension: u32, values: Vec<u8> },
}

impl Into<topk_rs::data::Matrix> for Matrix {
    fn into(self) -> topk_rs::data::Matrix {
        match self {
            Matrix::F32 { dimension, values } => topk_rs::data::Matrix::F32 { dimension, values },
            Matrix::U8 { dimension, values } => topk_rs::data::Matrix::U8 { dimension, values },
        }
    }
}
