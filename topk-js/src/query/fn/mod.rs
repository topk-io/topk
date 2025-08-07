use crate::{data::Value, expr::function::FunctionExpression};
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
        Value::Vector(vector) => Ok(FunctionExpression::vector_score(
            field,
            Value::Vector(vector),
        )),
        Value::SparseVector(vector) => Ok(FunctionExpression::vector_score(
            field,
            Value::SparseVector(vector),
        )),
        _ => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "Vector query must be a vector or sparse vector",
        )),
    }
}

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn bm25_score() -> FunctionExpression {
    FunctionExpression::keyword_score()
}

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpression {
    FunctionExpression::semantic_similarity(field, query)
}
