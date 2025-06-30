use crate::{
    data::Value,
    expr::function::{FunctionExpression, QueryVector},
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
        Value::Vector(query) => Ok(FunctionExpression::vector_score(
            field,
            QueryVector::Dense { query },
        )),
        Value::SparseVector(query) => Ok(FunctionExpression::vector_score(
            field,
            QueryVector::Sparse { query },
        )),
        v => Err(napi::Error::from_reason(format!("Unsupported vector query: {:?}", v)).into()),
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
