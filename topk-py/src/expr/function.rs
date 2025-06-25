use crate::data::vector::{SparseVector, Vector};
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: QueryVector },
    SemanticSimilarity { field: String, query: String },
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense(Vector),
    Sparse(SparseVector),
}

impl Into<topk_rs::expr::function::FunctionExpr> for FunctionExpr {
    fn into(self) -> topk_rs::expr::function::FunctionExpr {
        match self {
            FunctionExpr::KeywordScore {} => topk_rs::expr::function::FunctionExpr::KeywordScore {},
            FunctionExpr::VectorScore { field, query } => {
                topk_rs::expr::function::FunctionExpr::VectorScore {
                    field,
                    query: match query {
                        QueryVector::Dense(vector) => match vector {
                            Vector::F32(values) => topk_rs::data::Vector::F32(values).into(),
                            Vector::U8(values) => topk_rs::data::Vector::U8(values).into(),
                        },
                        QueryVector::Sparse(sparse_vector) => match sparse_vector {
                            SparseVector::F32 { indices, values } => {
                                topk_rs::data::SparseVector::F32 { indices, values }.into()
                            }
                            SparseVector::U8 { indices, values } => {
                                topk_rs::data::SparseVector::U8 { indices, values }.into()
                            }
                        },
                    },
                }
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_rs::expr::function::FunctionExpr::SemanticSimilarity { field, query }
            }
        }
    }
}
