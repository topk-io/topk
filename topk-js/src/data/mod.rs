pub mod collection;
pub mod document;
pub mod napi_box;
pub mod scalar;
pub mod sparse;
pub mod utils;
pub mod value;
pub mod vector;

use napi_derive::napi;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense {
        #[napi(ts_type = "data.Vector")]
        query: vector::Vector,
    },
    Sparse {
        #[napi(ts_type = "data.SparseVector")]
        query: sparse::SparseVector,
    },
}

impl Into<topk_rs::proto::v1::data::QueryVector> for QueryVector {
    fn into(self) -> topk_rs::proto::v1::data::QueryVector {
        match self {
            QueryVector::Dense { query } => {
                topk_rs::proto::v1::data::QueryVector::Dense(query.into())
            }
            QueryVector::Sparse { query } => {
                topk_rs::proto::v1::data::QueryVector::Sparse(query.into())
            }
        }
    }
}
