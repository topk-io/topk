use crate::{data::QueryVector, expr::function::FunctionExpression};
use napi_derive::napi;

pub mod query;
pub mod stage;

#[napi(namespace = "query")]
pub fn vector_distance(
    field: String,
    #[napi(ts_arg_type = "Array<number> | data.Vector")] query: QueryVector,
) -> FunctionExpression {
    FunctionExpression::VectorScore {
        field,
        query: query.into(),
    }
}

#[napi(namespace = "query")]
pub fn bm25_score() -> FunctionExpression {
    FunctionExpression::KeywordScore
}

#[napi(namespace = "query")]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpression {
    FunctionExpression::SemanticSimilarity { field, query }
}
