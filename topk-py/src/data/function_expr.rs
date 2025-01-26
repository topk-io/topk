use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpression {
    KeywordScore {},

    VectorScore { field: String, query: Vec<f32> },
}

impl Into<topk_protos::v1::data::FunctionExpr> for FunctionExpression {
    fn into(self) -> topk_protos::v1::data::FunctionExpr {
        match self {
            FunctionExpression::KeywordScore {} => {
                topk_protos::v1::data::FunctionExpr::bm25_score()
            }
            FunctionExpression::VectorScore { field, query } => {
                topk_protos::v1::data::FunctionExpr::vector_distance(
                    field,
                    topk_protos::v1::data::Vector::float(query),
                )
            }
        }
    }
}
