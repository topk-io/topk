use crate::proto::data::v1::{list, matrix};

mod document;
mod sparse_vector;
mod value;
use bytemuck::allocation::cast_vec;

// List values

pub trait IntoListValues {
    fn into_list_values(self) -> list::Values;
}

impl IntoListValues for Vec<u8> {
    fn into_list_values(self) -> list::Values {
        list::Values::U8(list::U8 { values: self })
    }
}

impl IntoListValues for Vec<u32> {
    fn into_list_values(self) -> list::Values {
        list::Values::U32(list::U32 { values: self })
    }
}

impl IntoListValues for Vec<u64> {
    fn into_list_values(self) -> list::Values {
        list::Values::U64(list::U64 { values: self })
    }
}

impl IntoListValues for Vec<i8> {
    fn into_list_values(self) -> list::Values {
        // Transmute to u8 and use `bytes` in proto because it doesn't have an i8 type
        let u8_vec = cast_vec(self);
        list::Values::I8(list::I8 { values: u8_vec })
    }
}

impl IntoListValues for Vec<i32> {
    fn into_list_values(self) -> list::Values {
        list::Values::I32(list::I32 { values: self })
    }
}

impl IntoListValues for Vec<i64> {
    fn into_list_values(self) -> list::Values {
        list::Values::I64(list::I64 { values: self })
    }
}

impl IntoListValues for Vec<float8::F8E4M3> {
    fn into_list_values(self) -> list::Values {
        list::Values::F8(list::F8 {
            values: cast_vec(self),
        })
    }
}

impl IntoListValues for Vec<half::f16> {
    fn into_list_values(self) -> list::Values {
        if self.is_empty() {
            return list::Values::F16(list::F16 {
                len: 0,
                values: vec![],
            });
        }

        list::Values::F16(list::F16 {
            len: self.len() as u32,
            values: convert_vec_f16_to_u32(self),
        })
    }
}

impl IntoListValues for Vec<f32> {
    fn into_list_values(self) -> list::Values {
        list::Values::F32(list::F32 { values: self })
    }
}

impl IntoListValues for Vec<f64> {
    fn into_list_values(self) -> list::Values {
        list::Values::F64(list::F64 { values: self })
    }
}

impl IntoListValues for Vec<String> {
    fn into_list_values(self) -> list::Values {
        list::Values::String(list::String { values: self })
    }
}

// Matrix values

pub trait IntoMatrixValues {
    fn into_matrix_values(self) -> matrix::Values;
}

impl IntoMatrixValues for Vec<f32> {
    fn into_matrix_values(self) -> matrix::Values {
        matrix::Values::F32(matrix::F32 { values: self })
    }
}

impl IntoMatrixValues for Vec<half::f16> {
    fn into_matrix_values(self) -> matrix::Values {
        if self.is_empty() {
            return matrix::Values::F16(matrix::F16 {
                len: 0,
                values: vec![],
            });
        }

        matrix::Values::F16(matrix::F16 {
            len: self.len() as u32,
            values: convert_vec_f16_to_u32(self),
        })
    }
}

impl IntoMatrixValues for Vec<float8::F8E4M3> {
    fn into_matrix_values(self) -> matrix::Values {
        matrix::Values::F8(matrix::F8 {
            values: cast_vec(self),
        })
    }
}

impl IntoMatrixValues for Vec<u8> {
    fn into_matrix_values(self) -> matrix::Values {
        matrix::Values::U8(matrix::U8 { values: self })
    }
}

impl IntoMatrixValues for Vec<i8> {
    fn into_matrix_values(self) -> matrix::Values {
        matrix::Values::I8(matrix::I8 {
            values: cast_vec(self),
        })
    }
}

#[inline]
fn convert_vec_f16_to_u32(mut values: Vec<half::f16>) -> Vec<u32> {
    // Resize to the nearest multiple of 2
    let len = values.len();
    let aligned_len = len.next_multiple_of(2);
    values.resize(aligned_len, half::f16::ZERO);
    values.shrink_to_fit();

    // If the vector is aligned to 4 bytes, we can use the vector directly
    if (values.as_ptr() as usize) % 4 == 0 {
        // Break the vector into parts
        let cap = values.capacity();
        let ptr = values.as_mut_ptr();
        std::mem::forget(values);

        // Reconstruct u32 from f16 parts
        unsafe {
            // SAFETY
            // Copying len(self) f16 into u32 with capacity for ceil(len(self) / 2) u32s.
            Vec::from_raw_parts(ptr as *mut u32, aligned_len / 2, cap / 2)
        }
    } else {
        // Copy f16 to an 4-byte aligned vector
        let mut out = vec![0u32; aligned_len / 2];
        unsafe {
            // SAFETY
            // Copying len(self) f16 into u32 with capacity for ceil(len(self) / 2) u32s.
            std::ptr::copy_nonoverlapping(values.as_ptr(), out.as_mut_ptr() as *mut half::f16, len);
        }
        out
    }
}
