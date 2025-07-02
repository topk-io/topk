mod dense;
mod sparse;

pub use dense::Vector;
pub(crate) use dense::{F32Vector, U8Vector};
pub use sparse::SparseVector;
pub(crate) use sparse::{F32SparseVector, U8SparseVector};
