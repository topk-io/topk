use crate::data::value::Value;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore {
        field: String,
        query: Value,
        skip_refine: bool,
    },
    SemanticSimilarity {
        field: String,
        query: String,
    },
}

impl From<FunctionExpr> for topk_rs::proto::v1::data::FunctionExpr {
    fn from(expr: FunctionExpr) -> Self {
        match expr {
            FunctionExpr::KeywordScore {} => topk_rs::proto::v1::data::FunctionExpr::bm25_score(),
            FunctionExpr::VectorScore {
                field,
                query,
                skip_refine,
            } => topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query, skip_refine),
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
