use crate::{
    data::Value,
    expr::function::{FunctionExpression, FunctionExpressionUnion},
};
use napi_derive::napi;

#[napi(object, namespace = "query_fn")]
#[derive(Default)]
pub struct VectorDistanceOptions {
    pub skip_refine: Option<bool>,
}

#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn vector_distance(
    field: String,
    #[napi(ts_arg_type = "Array<number> | Record<number, number> | data.List | data.SparseVector")]
    query: Value,
    options: Option<VectorDistanceOptions>,
) -> napi::Result<FunctionExpression> {
    let skip_refine = options.and_then(|o| o.skip_refine).unwrap_or(false);
    match query {
        Value::List(list) => Ok(FunctionExpression(FunctionExpressionUnion::VectorScore {
            field,
            query: Value::List(list),
            skip_refine,
        })),
        Value::SparseVector(query) => {
            Ok(FunctionExpression(FunctionExpressionUnion::VectorScore {
                field,
                query: Value::SparseVector(query),
                skip_refine,
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
