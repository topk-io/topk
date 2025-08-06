use crate::proto::{
    data::v1::Value,
    v1::data::{function_expr, FunctionExpr},
};

impl FunctionExpr {
    pub fn vector_distance(field: impl Into<String>, query: impl Into<Value>) -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::VectorDistance(
                function_expr::VectorDistance {
                    field: field.into(),
                    query: Some(query.into()),
                    #[allow(deprecated)]
                    dense_query: None,
                    #[allow(deprecated)]
                    sparse_query: None,
                },
            )),
        }
    }

    pub fn bm25_score() -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::Bm25Score(function_expr::Bm25Score {})),
        }
    }

    pub fn semantic_similarity(field: impl Into<String>, query: impl Into<String>) -> Self {
        FunctionExpr {
            func: Some(function_expr::Func::SemanticSimilarity(
                function_expr::SemanticSimilarity {
                    field: field.into(),
                    query: query.into(),
                },
            )),
        }
    }
}
