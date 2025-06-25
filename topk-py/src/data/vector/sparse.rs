use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum SparseVector {
    F32 { indices: Vec<u32>, values: Vec<f32> },
    U8 { indices: Vec<u32>, values: Vec<u8> },
}

impl Into<topk_rs::data::SparseVector> for SparseVector {
    fn into(self) -> topk_rs::data::SparseVector {
        match self {
            SparseVector::F32 { indices, values } => {
                topk_rs::data::SparseVector::F32 { indices, values }
            }
            SparseVector::U8 { indices, values } => {
                topk_rs::data::SparseVector::U8 { indices, values }
            }
        }
    }
}
