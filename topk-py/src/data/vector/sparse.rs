use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, PartialEq, Clone)]
pub enum SparseVector {
    F32 { indices: Vec<u32>, values: Vec<f32> },
    U8 { indices: Vec<u32>, values: Vec<u8> },
}

impl From<SparseVector> for topk_rs::proto::v1::data::SparseVector {
    fn from(sparse: SparseVector) -> Self {
        match sparse {
            SparseVector::F32 { indices, values } => {
                topk_rs::proto::v1::data::SparseVector::f32(indices, values)
            }
            SparseVector::U8 { indices, values } => {
                topk_rs::proto::v1::data::SparseVector::u8(indices, values)
            }
        }
    }
}
