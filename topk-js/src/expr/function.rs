use napi_derive::napi;

#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct FunctionExpression(FunctionExpressionUnion);

impl FunctionExpression {
    pub(crate) fn keyword_score() -> Self {
        FunctionExpression(FunctionExpressionUnion::KeywordScore)
    }

    pub(crate) fn vector_score(field: String, query: QueryVector) -> Self {
        FunctionExpression(FunctionExpressionUnion::VectorScore { field, query })
    }

    pub(crate) fn semantic_similarity(field: String, query: String) -> Self {
        FunctionExpression(FunctionExpressionUnion::SemanticSimilarity { field, query })
    }
}

#[derive(Debug, Clone)]
pub enum FunctionExpressionUnion {
    KeywordScore,
    VectorScore { field: String, query: QueryVector },
    SemanticSimilarity { field: String, query: String },
}

impl Into<topk_rs::expr::function::FunctionExpr> for FunctionExpression {
    fn into(self) -> topk_rs::expr::function::FunctionExpr {
        match self.0 {
            FunctionExpressionUnion::KeywordScore => {
                topk_rs::expr::function::FunctionExpr::KeywordScore {}
            }
            FunctionExpressionUnion::VectorScore { field, query } => {
                topk_rs::expr::function::FunctionExpr::VectorScore {
                    field,
                    query: query.into(),
                }
            }
            FunctionExpressionUnion::SemanticSimilarity { field, query } => {
                topk_rs::expr::function::FunctionExpr::SemanticSimilarity { field, query }
            }
        }
    }
}

impl Into<topk_rs::proto::v1::data::FunctionExpr> for FunctionExpressionUnion {
    fn into(self) -> topk_rs::proto::v1::data::FunctionExpr {
        match self {
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

#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense { query: crate::data::Vector },
    Sparse { query: crate::data::SparseVector },
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
