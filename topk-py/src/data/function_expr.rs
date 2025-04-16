use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum VectorQuery {
    F32(Vec<f32>),
    U8(Vec<u8>),
}

impl Into<topk_rs::data::function_expr::Vector> for VectorQuery {
    fn into(self) -> topk_rs::data::function_expr::Vector {
        match self {
            VectorQuery::F32(values) => topk_rs::data::function_expr::Vector::float(values),
            VectorQuery::U8(values) => topk_rs::data::function_expr::Vector::byte(values),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: VectorQuery },
    SemanticSimilarity { field: String, query: String },
}

impl Into<topk_rs::data::function_expr::FunctionExpr> for FunctionExpr {
    fn into(self) -> topk_rs::data::function_expr::FunctionExpr {
        match self {
            FunctionExpr::KeywordScore {} => {
                topk_rs::data::function_expr::FunctionExpr::KeywordScore {}
            }
            FunctionExpr::VectorScore { field, query } => {
                topk_rs::data::function_expr::FunctionExpr::VectorScore {
                    field,
                    query: query.into(),
                }
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_rs::data::function_expr::FunctionExpr::SemanticSimilarity { field, query }
            }
        }
    }
}
