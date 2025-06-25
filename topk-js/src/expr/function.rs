use crate::data::QueryVector;
use napi_derive::napi;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub enum FunctionExpression {
    KeywordScore,
    VectorScore {
        field: String,
        #[napi]
        query: QueryVector,
    },
    SemanticSimilarity {
        field: String,
        query: String,
    },
}

impl Into<topk_rs::expr::function::FunctionExpr> for FunctionExpression {
    fn into(self) -> topk_rs::expr::function::FunctionExpr {
        match self {
            FunctionExpression::KeywordScore => {
                topk_rs::expr::function::FunctionExpr::KeywordScore {}
            }
            FunctionExpression::VectorScore { field, query } => {
                topk_rs::expr::function::FunctionExpr::VectorScore {
                    field,
                    query: query.into(),
                }
            }
            FunctionExpression::SemanticSimilarity { field, query } => {
                topk_rs::expr::function::FunctionExpr::SemanticSimilarity { field, query }
            }
        }
    }
}

impl Into<topk_rs::proto::v1::data::FunctionExpr> for FunctionExpression {
    fn into(self) -> topk_rs::proto::v1::data::FunctionExpr {
        match self {
            FunctionExpression::KeywordScore => {
                topk_rs::proto::v1::data::FunctionExpr::bm25_score()
            }
            FunctionExpression::VectorScore { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::vector_distance(field, query)
            }
            FunctionExpression::SemanticSimilarity { field, query } => {
                topk_rs::proto::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
