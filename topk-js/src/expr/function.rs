use napi_derive::napi;

use crate::data::Value;

/// @internal
/// @hideconstructor
#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct FunctionExpression(pub(crate) FunctionExpressionUnion);

#[derive(Debug, Clone)]
pub enum FunctionExpressionUnion {
    KeywordScore,
    VectorScore {
        field: String,
        query: Value,
        skip_refine: bool,
    },
    MultiVectorDistance {
        field: String,
        query: Value,
        candidates: Option<u32>,
    },
    SemanticSimilarity {
        field: String,
        query: String,
    },
}

impl From<FunctionExpression> for topk_rs::proto::v1::data::FunctionExpr {
    fn from(expr: FunctionExpression) -> Self {
        match expr.0 {
            FunctionExpressionUnion::KeywordScore => {
                topk_rs::proto::v1::data::FunctionExpr::bm25_score()
            }
            FunctionExpressionUnion::VectorScore {
                field,
                query,
                skip_refine,
            } => topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query, skip_refine),
            FunctionExpressionUnion::MultiVectorDistance {
                field,
                query,
                candidates,
            } => topk_rs::proto::v1::data::FunctionExpr::multi_vector_distance(field, query, candidates),
            FunctionExpressionUnion::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
