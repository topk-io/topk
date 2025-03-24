use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub enum VectorQuery {
  F32 { vector: Vec<f64> },
  U8 { vector: Vec<u8> },
}

impl Into<topk_protos::v1::data::Vector> for VectorQuery {
  fn into(self) -> topk_protos::v1::data::Vector {
    match self {
      VectorQuery::F32 { vector } => {
        // todo: check if f64 -> f32 is lossy
        topk_protos::v1::data::Vector::float(vector.into_iter().map(|v| v as f32).collect())
      }
      VectorQuery::U8 { vector } => topk_protos::v1::data::Vector::byte(vector),
    }
  }
}

#[napi]
pub enum FunctionExpression {
  KeywordScore,
  VectorScore { field: String, query: VectorQuery },
  SemanticSimilarity { field: String, query: String },
}

impl Into<topk_protos::v1::data::FunctionExpr> for FunctionExpression {
  fn into(self) -> topk_protos::v1::data::FunctionExpr {
    match self {
      FunctionExpression::KeywordScore {} => topk_protos::v1::data::FunctionExpr::bm25_score(),
      FunctionExpression::VectorScore { field, query } => {
        topk_protos::v1::data::FunctionExpr::vector_distance(field, query.into())
      }
      FunctionExpression::SemanticSimilarity { field, query } => {
        topk_protos::v1::data::FunctionExpr::semantic_similarity(field, query)
      }
    }
  }
}
