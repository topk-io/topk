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

#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense { query: crate::data::Vector },
    Sparse { query: crate::data::SparseVector },
}

impl From<QueryVector> for topk_rs::proto::v1::data::Value {
    fn from(query: QueryVector) -> Self {
        match query {
            QueryVector::Dense { query } => query.into(),
            QueryVector::Sparse { query } => topk_rs::proto::v1::data::Value {
                value: Some(topk_rs::proto::v1::data::value::Value::SparseVector(
                    query.into(),
                )),
            },
        }
    }
}
