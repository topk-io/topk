include!(concat!(env!("OUT_DIR"), "/topk.data.v1.rs"));

mod document_ext;
mod query_ext;
mod value_ext;

#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense(Vector),
    Sparse(SparseVector),
}

impl From<Vector> for QueryVector {
    fn from(vector: Vector) -> Self {
        QueryVector::Dense(vector)
    }
}

impl From<SparseVector> for QueryVector {
    fn from(sparse_vector: SparseVector) -> Self {
        QueryVector::Sparse(sparse_vector)
    }
}

impl From<Vec<f32>> for QueryVector {
    fn from(values: Vec<f32>) -> Self {
        QueryVector::Dense(Vector::float(values))
    }
}

impl From<Vec<u8>> for QueryVector {
    fn from(values: Vec<u8>) -> Self {
        QueryVector::Dense(Vector::byte(values))
    }
}
