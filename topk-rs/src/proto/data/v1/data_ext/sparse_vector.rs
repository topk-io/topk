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
    
    /// Creates a F32 sparse vector with validation that indices are sorted
    pub fn f32_validated(indices: Vec<u32>, values: Vec<f32>) -> Result<Self, String> {
        if indices.len() != values.len() {
            return Err("indices and values must have the same length".to_string());
        }
        
        // Validate that indices are sorted
        for i in 1..indices.len() {
            if indices[i] <= indices[i - 1] {
                return Err("indices must be sorted in ascending order and unique".to_string());
            }
        }
        
        Ok(SparseVector {
            indices,
            values: Some(sparse_vector::Values::F32(sparse_vector::F32Values {
                values,
            })),
        })
    }

    pub fn u8(indices: Vec<u32>, values: Vec<u8>) -> Self {
        SparseVector {
            indices,
            values: Some(sparse_vector::Values::U8(sparse_vector::U8Values {
                values,
            })),
        }
    }
    
    /// Creates a U8 sparse vector with validation that indices are sorted
    pub fn u8_validated(indices: Vec<u32>, values: Vec<u8>) -> Result<Self, String> {
        if indices.len() != values.len() {
            return Err("indices and values must have the same length".to_string());
        }
        
        // Validate that indices are sorted
        for i in 1..indices.len() {
            if indices[i] <= indices[i - 1] {
                return Err("indices must be sorted in ascending order and unique".to_string());
            }
        }
        
        Ok(SparseVector {
            indices,
            values: Some(sparse_vector::Values::U8(sparse_vector::U8Values {
                values,
            })),
        })
    }
}
