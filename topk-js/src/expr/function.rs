use napi_derive::napi;

use crate::data::Value;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct FunctionExpression(pub(crate) FunctionExpressionUnion);

#[derive(Debug, Clone)]
pub enum FunctionExpressionUnion {
    KeywordScore,
    VectorScore { field: String, query: Value },
    SemanticSimilarity { field: String, query: String },
}

impl From<FunctionExpression> for topk_rs::proto::v1::data::FunctionExpr {
    fn from(expr: FunctionExpression) -> Self {
        match expr.0 {
            FunctionExpressionUnion::KeywordScore => {
                topk_rs::proto::v1::data::FunctionExpr::bm25_score()
            }
            FunctionExpressionUnion::VectorScore { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query)
            }
            FunctionExpressionUnion::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
