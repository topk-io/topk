mod dense;
mod sparse;

pub use dense::Vector;
pub(crate) use dense::VectorData;
pub(crate) use dense::VectorUnion;

pub use sparse::SparseVector;
pub(crate) use sparse::SparseVectorData;
pub(crate) use sparse::SparseVectorUnion;
