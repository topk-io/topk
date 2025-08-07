use crate::{
    data::Value,
    expr::function::{FunctionExpression, FunctionExpressionUnion},
};
use napi_derive::napi;

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn vector_distance(
    field: String,
    #[napi(
        ts_arg_type = "Array<number> | Record<number, number> | data.Vector | data.SparseVector"
    )]
    query: Value,
) -> napi::Result<FunctionExpression> {
    match query {
        Value::Vector(vector) => Ok(FunctionExpression(FunctionExpressionUnion::VectorScore {
            field,
            query: Value::Vector(vector),
        })),
        Value::SparseVector(vector) => {
            Ok(FunctionExpression(FunctionExpressionUnion::VectorScore {
                field,
                query: Value::SparseVector(vector),
            }))
        }
        _ => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "Vector query must be a vector or sparse vector",
        )),
    }
}

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn bm25_score() -> FunctionExpression {
    FunctionExpression(FunctionExpressionUnion::KeywordScore)
}

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpression {
    FunctionExpression(FunctionExpressionUnion::SemanticSimilarity { field, query })
}
