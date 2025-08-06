use crate::proto::{
    data::v1::{list, value, SparseVector, Value, Vector},
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
        #[allow(deprecated)]
        QueryVector::Dense(Vector::f32(values))
    }
}

impl From<Vec<u8>> for QueryVector {
    fn from(values: Vec<u8>) -> Self {
        #[allow(deprecated)]
        QueryVector::Dense(Vector::u8(values))
    }
}

impl From<Value> for QueryVector {
    fn from(value: Value) -> Self {
        match value.value {
            Some(value::Value::Vector(vector)) => QueryVector::Dense(vector),
            Some(value::Value::SparseVector(sparse_vector)) => QueryVector::Sparse(sparse_vector),
            Some(value::Value::List(list)) => match list.values {
                #[allow(deprecated)]
                Some(list::Values::F32(f32)) => QueryVector::Dense(Vector::f32(f32.values)),
                #[allow(deprecated)]
                Some(list::Values::U8(u8)) => QueryVector::Dense(Vector::u8(u8.values)),
                Some(_) => {
                    unimplemented!("Only list<f32> and list<u8> can be converted to QueryVector")
                }
                None => unimplemented!("List cannot be empty"),
            },
            _ => {
                unimplemented!("Only Vector, SparseVector and List can be converted to QueryVector")
            }
        }
    }
}
