use crate::data::value::Value;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {
        b: Option<f32>,
        k1: Option<f32>,
    },
    VectorScore {
        field: String,
        query: Value,
        skip_refine: bool,
    },
    SemanticSimilarity {
        field: String,
        query: String,
    },
    MultiVectorDistance {
        field: String,
        query: Value,
        candidates: Option<u32>,
    },
}

impl From<FunctionExpr> for topk_rs::proto::v1::data::FunctionExpr {
    fn from(expr: FunctionExpr) -> Self {
        match expr {
            FunctionExpr::KeywordScore { b, k1 } => {
                topk_rs::proto::v1::data::FunctionExpr::bm25_score(b, k1)
            }
            FunctionExpr::VectorScore {
                field,
                query,
                skip_refine,
            } => topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query, skip_refine),
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
            FunctionExpr::MultiVectorDistance {
                field,
                query,
                candidates,
            } => topk_rs::proto::v1::data::FunctionExpr::multi_vector_distance(
                field, query, candidates,
            ),
        }
    }
}
