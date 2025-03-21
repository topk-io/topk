use topk_protos::v1::data;

pub enum VectorQuery {
  F32(Vec<f32>),
  U8(Vec<u8>),
}

impl Into<topk_protos::v1::data::Vector> for VectorQuery {
  fn into(self) -> topk_protos::v1::data::Vector {
    match self {
      VectorQuery::F32(values) => topk_protos::v1::data::Vector::float(values),
      VectorQuery::U8(values) => topk_protos::v1::data::Vector::byte(values),
    }
  }
}

pub enum FunctionExpression {
  KeywordScore {},
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
