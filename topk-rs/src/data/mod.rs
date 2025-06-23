mod scalar;
pub use scalar::Scalar;

mod vector;
pub use vector::Vector;

mod sparse_vector;
pub use sparse_vector::SparseVector;

pub fn float_vector(values: Vec<f32>) -> Vector {
    Vector::F32(values)
}

pub fn byte_vector(values: Vec<u8>) -> Vector {
    Vector::U8(values)
}
