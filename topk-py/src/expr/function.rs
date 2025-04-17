use crate::data::vector::Vector;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: Vector },
    SemanticSimilarity { field: String, query: String },
}

impl Into<topk_rs::expr::function::FunctionExpr> for FunctionExpr {
    fn into(self) -> topk_rs::expr::function::FunctionExpr {
        match self {
            FunctionExpr::KeywordScore {} => topk_rs::expr::function::FunctionExpr::KeywordScore {},
            FunctionExpr::VectorScore { field, query } => {
                topk_rs::expr::function::FunctionExpr::VectorScore {
                    field,
                    query: match query {
                        Vector::F32(values) => topk_rs::data::Vector::F32(values),
                        Vector::U8(values) => topk_rs::data::Vector::U8(values),
                    },
                }
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_rs::expr::function::FunctionExpr::SemanticSimilarity { field, query }
            }
        }
    }
}
