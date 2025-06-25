#[derive(Debug, Clone)]
pub enum SparseVector {
    F32 { indices: Vec<u32>, values: Vec<f32> },
    U8 { indices: Vec<u32>, values: Vec<u8> },
}

impl Into<topk_protos::v1::data::SparseVector> for SparseVector {
    fn into(self) -> topk_protos::v1::data::SparseVector {
        match self {
            SparseVector::F32 { indices, values } => topk_protos::v1::data::SparseVector {
                indices,
                values: Some(topk_protos::v1::data::sparse_vector::Values::F32(
                    topk_protos::v1::data::sparse_vector::F32Values { values },
                )),
            },
            SparseVector::U8 { indices, values } => topk_protos::v1::data::SparseVector {
                indices,
                values: Some(topk_protos::v1::data::sparse_vector::Values::U8(
                    topk_protos::v1::data::sparse_vector::U8Values { values },
                )),
            },
        }
    }
}

impl From<topk_protos::v1::data::SparseVector> for SparseVector {
    fn from(sparse_vector: topk_protos::v1::data::SparseVector) -> Self {
        match sparse_vector.values {
            Some(topk_protos::v1::data::sparse_vector::Values::F32(values)) => SparseVector::F32 {
                indices: sparse_vector.indices,
                values: values.values,
            },
            Some(topk_protos::v1::data::sparse_vector::Values::U8(values)) => SparseVector::U8 {
                indices: sparse_vector.indices,
                values: values.values,
            },
            t => panic!("Invalid sparse vector type: {:?}", t),
        }
    }
}

impl From<SparseVector> for topk_protos::v1::data::QueryVector {
    fn from(vector: SparseVector) -> Self {
        topk_protos::v1::data::QueryVector::Sparse(vector.into())
    }
}
