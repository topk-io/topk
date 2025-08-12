use crate::{
    data::Value,
    expr::function::{FunctionExpression, FunctionExpressionUnion},
};
use napi_derive::napi;

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn vector_distance(
    field: String,
    #[napi(ts_arg_type = "Array<number> | Record<number, number> | data.List | data.SparseVector")]
    query: Value,
) -> napi::Result<FunctionExpression> {
    match query {
        Value::List(list) => Ok(FunctionExpression(FunctionExpressionUnion::VectorScore {
            field,
            query: Value::List(list),
        })),
        Value::SparseVector(query) => {
            Ok(FunctionExpression(FunctionExpressionUnion::VectorScore {
                field,
                query: Value::SparseVector(query),
            }))
        }
        v => Err(napi::Error::from_reason(format!("Unsupported vector query: {:?}", v)).into()),
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
