mod dense;
mod sparse;

pub use dense::Vector;
pub(crate) use dense::VectorUnion;
pub(crate) use sparse::SparseVectorUnion;

pub use sparse::SparseVector;
pub(crate) use sparse::SparseVectorData;
