use crate::proto::{
    data::v1::{SparseVector, Vector},
    v1::data::{function_expr, FunctionExpr},
};

impl FunctionExpr {
    pub fn vector_distance(field: impl Into<String>, query: impl Into<QueryVector>) -> Self {
        let dist = match query.into() {
            QueryVector::Dense(query) => function_expr::VectorDistance {
                field: field.into(),
                query: Some(query),
                sparse_query: None,
            },
            QueryVector::Sparse(sparse_query) => function_expr::VectorDistance {
                field: field.into(),
                query: None,
                sparse_query: Some(sparse_query),
            },
        };
        FunctionExpr {
            func: Some(function_expr::Func::VectorDistance(dist)),
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

#[derive(Debug, Clone)]
pub enum QueryVector {
    Dense(Vector),
    Sparse(SparseVector),
}

impl From<Vector> for QueryVector {
    fn from(vector: Vector) -> Self {
        QueryVector::Dense(vector)
    }
}

impl From<SparseVector> for QueryVector {
    fn from(sparse_vector: SparseVector) -> Self {
        QueryVector::Sparse(sparse_vector)
    }
}

impl From<Vec<f32>> for QueryVector {
    fn from(values: Vec<f32>) -> Self {
        QueryVector::Dense(Vector::f32(values))
    }
}

impl From<Vec<u8>> for QueryVector {
    fn from(values: Vec<u8>) -> Self {
        QueryVector::Dense(Vector::u8(values))
    }
}
