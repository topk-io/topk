use crate::data::vector::{SparseVector, Vector};
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: QueryVector },
    SemanticSimilarity { field: String, query: String },
}

impl From<FunctionExpr> for topk_rs::proto::v1::data::FunctionExpr {
    fn from(expr: FunctionExpr) -> Self {
        match expr {
            FunctionExpr::KeywordScore {} => topk_rs::proto::v1::data::FunctionExpr::bm25_score(),
            FunctionExpr::VectorScore { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query)
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense(Vector),
    Sparse(SparseVector),
}

impl From<QueryVector> for topk_rs::proto::v1::data::QueryVector {
    fn from(query: QueryVector) -> Self {
        match query {
            QueryVector::Dense(vector) => {
                topk_rs::proto::v1::data::QueryVector::Dense(vector.into())
            }
            QueryVector::Sparse(sparse) => {
                topk_rs::proto::v1::data::QueryVector::Sparse(sparse.into())
            }
        }
    }
}
