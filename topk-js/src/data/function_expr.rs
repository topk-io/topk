use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::value::Vector;

#[napi]
#[derive(Debug, Clone)]
pub enum FunctionExpression {
    KeywordScore,
    VectorScore { field: String, query: VectorQuery },
    SemanticSimilarity { field: String, query: String },
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

#[napi]
#[derive(Debug, Clone)]
pub enum VectorQuery {
    F32 { vector: Vec<f64> },
    U8 { vector: Vec<u8> },
}

impl Into<topk_protos::v1::data::Vector> for VectorQuery {
    fn into(self) -> topk_protos::v1::data::Vector {
        match self {
            VectorQuery::F32 { vector } => {
                topk_protos::v1::data::Vector::float(vector.iter().map(|v| *v as f32).collect())
            }
            VectorQuery::U8 { vector } => topk_protos::v1::data::Vector::byte(vector),
        }
    }
}

impl Into<topk_rs::data::Vector> for VectorQuery {
    fn into(self) -> topk_rs::data::Vector {
        match self {
            VectorQuery::F32 { vector } => {
                topk_rs::data::Vector::F32(vector.iter().map(|v| *v as f32).collect())
            }
            VectorQuery::U8 { vector } => topk_rs::data::Vector::U8(vector),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VectorDistanceQuery {
    Vector { vector: Vector },
}

impl FromNapiValue for VectorDistanceQuery {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        unsafe {
            let mut is_array: bool = false;
            napi::sys::napi_is_array(env, value, &mut is_array);

            if is_array {
                // If it's a JS array, convert to Vec<f64> and then to VectorDistanceQuery::Array
                let arr: Vec<f64> = Vec::from_napi_value(env, value)?;

                return Ok(VectorDistanceQuery::Vector {
                    vector: Vector::Float { values: arr },
                });
            } else {
                match Vector::from_napi_value(env, value) {
                    Ok(vector) => Ok(VectorDistanceQuery::Vector { vector }),
                    Err(_) => {
                        // If all else fails, try to convert to an array
                        let arr: Vec<f64> = Vec::from_napi_value(env, value)?;
                        Ok(VectorDistanceQuery::Vector {
                            vector: Vector::Float { values: arr },
                        })
                    }
                }
            }
        }
    }
}

#[napi(namespace = "query")]
pub fn vector_distance(
    field: String,
    #[napi(ts_arg_type = "Array<number> | Vector")] query: VectorDistanceQuery,
) -> FunctionExpression {
    FunctionExpression::VectorScore {
        field,
        query: match query {
            VectorDistanceQuery::Vector { vector } => match vector {
                Vector::Float { values } => VectorQuery::F32 { vector: values },
                Vector::Byte { values } => VectorQuery::U8 { vector: values },
                Vector::Binary { values } => VectorQuery::U8 { vector: values },
            },
        },
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

impl Into<topk_protos::v1::data::FunctionExpr> for FunctionExpression {
    fn into(self) -> topk_protos::v1::data::FunctionExpr {
        match self {
            FunctionExpression::KeywordScore => topk_protos::v1::data::FunctionExpr::bm25_score(),
            FunctionExpression::VectorScore { field, query } => {
                topk_protos::v1::data::FunctionExpr::vector_distance(field, query.into())
            }
            FunctionExpression::SemanticSimilarity { field, query } => {
                topk_protos::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
