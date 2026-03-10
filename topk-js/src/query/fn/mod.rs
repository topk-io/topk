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

/// Computes the vector distance between a field and a query vector.
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

/// Options for BM25 scoring.
#[napi(object, namespace = "query_fn")]
#[derive(Default)]
pub struct Bm25ScoreOptions {
    /// BM25 parameter b (0-1)
    pub b: Option<f64>,
    /// BM25 parameter k1 (>= 0)
    pub k1: Option<f64>,
}

/// Computes the BM25 score for a keyword search.
#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn bm25_score(options: Option<Bm25ScoreOptions>) -> napi::Result<FunctionExpression> {
    let opts = options.unwrap_or_default();
    if let Some(b) = opts.b {
        if b < 0.0 || b > 1.0 {
            return Err(napi::Error::from_reason("b must be between 0.0 and 1.0"));
        }
    }
    if let Some(k1) = opts.k1 {
        if k1 < 0.0 {
            return Err(napi::Error::from_reason("k1 must be >= 0.0"));
        }
    }
    Ok(FunctionExpression(FunctionExpressionUnion::KeywordScore {
        b: opts.b.map(|v| v as f32),
        k1: opts.k1.map(|v| v as f32),
    }))
}

/// Computes the semantic similarity between a field and a query string.
#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpression {
    FunctionExpression(FunctionExpressionUnion::SemanticSimilarity { field, query })
}

/// Calculate the multi-vector distance between a field and a query matrix.
///
/// The query matrix can be an array of number arrays (defaults to f32),
/// or a [`Matrix`](https://docs.topk.io/sdk/topk-js/data#Matrix) instance. To specify a different matrix type,
/// use [`matrix()`](https://docs.topk.io/sdk/topk-js/data#matrix) with `valueType`.
///
/// The optional `candidates` parameter limits the number of candidate vectors considered during retrieval.
///
/// ```javascript
/// import { field, fn, select } from "topk-js/query";
///
/// client.collection("books").query(
///   select({
///     title: field("title"),
///     title_distance: fn.multiVectorDistance(
///       "title_embedding",
///       [[0.1, 0.2, 0.3, ...], [0.4, 0.5, 0.6, ...]],
///       100
///     )
///   })
///   .topk(field("title_distance"), 10)
/// )
/// ```
#[napi(namespace = "query_fn", ts_return_type = "query.FunctionExpression")]
pub fn multi_vector_distance(
    field: String,
    #[napi(ts_arg_type = "Array<Array<number>> | data.Matrix")] query: Value,
    candidates: Option<u32>,
) -> napi::Result<FunctionExpression> {
    match query {
        Value::Matrix(_) => Ok(FunctionExpression(
            FunctionExpressionUnion::MultiVectorDistance {
                field,
                query,
                candidates,
            },
        )),
        v => Err(napi::Error::from_reason(format!(
            "Multi-vector query must be a matrix value, got: {:?}",
            v
        ))
        .into()),
    }
}
