use crate::data::Vector;

#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: Vector },
    SemanticSimilarity { field: String, query: String },
}

impl Into<topk_protos::v1::data::FunctionExpr> for FunctionExpr {
    fn into(self) -> topk_protos::v1::data::FunctionExpr {
        match self {
            FunctionExpr::KeywordScore {} => topk_protos::v1::data::FunctionExpr::bm25_score(),
            FunctionExpr::VectorScore { field, query } => {
                topk_protos::v1::data::FunctionExpr::vector_distance(field, query.into())
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_protos::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
