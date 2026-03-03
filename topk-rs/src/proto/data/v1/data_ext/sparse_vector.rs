use crate::proto::data::v1::{sparse_vector, SparseVector};
use crate::Error;

impl SparseVector {
    pub fn values(&self) -> Result<&sparse_vector::Values, Error> {
        self.values.as_ref().ok_or(Error::InvalidProto)
    }

    pub fn f32(indices: Vec<u32>, values: Vec<f32>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::F32(sparse_vector::F32Values {
                values,
            })),
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
}
