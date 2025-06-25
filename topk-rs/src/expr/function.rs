use topk_protos::v1::data::QueryVector;

#[derive(Debug, Clone)]
pub enum FunctionExpr {
    KeywordScore {},
    VectorScore { field: String, query: QueryVector },
    SemanticSimilarity { field: String, query: String },
}

impl FunctionExpr {
    /// Computes BM25 score for documents matching text filter.
    pub fn bm25_score() -> Self {
        Self::KeywordScore {}
    }

    /// Computes vector distance for the given `field` and `query` vector. The field must
    /// have a vector index specified in the schema
    pub fn vector_distance(field: impl Into<String>, query: impl Into<QueryVector>) -> Self {
        Self::VectorScore {
            field: field.into(),
            query: query.into(),
        }
    }

    /// Computes semantic similarity for the given `field` and `query` string. The `field` must
    /// have a semantic index specified in the schema.
    pub fn semantic_similarity(field: impl Into<String>, query: impl Into<String>) -> Self {
        Self::SemanticSimilarity {
            field: field.into(),
            query: query.into(),
        }
    }
}

impl Into<topk_protos::v1::data::FunctionExpr> for FunctionExpr {
    fn into(self) -> topk_protos::v1::data::FunctionExpr {
        match self {
            FunctionExpr::KeywordScore {} => topk_protos::v1::data::FunctionExpr::bm25_score(),
            FunctionExpr::VectorScore { field, query } => {
                topk_protos::v1::data::FunctionExpr::vector_distance(field, query)
            }
            FunctionExpr::SemanticSimilarity { field, query } => {
                topk_protos::v1::data::FunctionExpr::semantic_similarity(field, query)
            }
        }
    }
}
