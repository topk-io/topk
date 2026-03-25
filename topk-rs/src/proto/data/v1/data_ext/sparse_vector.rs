use float8::F8E4M3;

use crate::proto::data::v1::{sparse_vector, SparseVector};

impl SparseVector {
    pub fn f32(indices: Vec<u32>, values: Vec<f32>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::F32(sparse_vector::F32Values {
                values,
            })),
        }
    }

    pub fn f16(indices: Vec<u32>, values: Vec<half::f16>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::F16(values.into())),
        }
    }

    pub fn f8(indices: Vec<u32>, values: Vec<F8E4M3>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::F8(values.into())),
        }
    }

    pub fn u8(indices: Vec<u32>, values: Vec<u8>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::U8(sparse_vector::U8Values {
                values,
            })),
        }
    }

    pub fn i8(indices: Vec<u32>, values: Vec<i8>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::I8(values.into())),
        }
    }
}
