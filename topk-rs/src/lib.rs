pub mod proto;

pub mod error;
pub use error::Error;

mod client;
pub use client::Client;
pub use client::ClientConfig;
pub use client::CollectionClient;
pub use client::CollectionsClient;

pub mod defaults {
    pub use crate::client::RETRY_BACKOFF_BASE;
    pub use crate::client::RETRY_BACKOFF_INIT;
    pub use crate::client::RETRY_BACKOFF_MAX;
    pub use crate::client::RETRY_MAX_RETRIES;
    pub use crate::client::RETRY_TIMEOUT;
}

pub use client::retry;

// Public API
pub mod data {
    use crate::proto::v1::data::{SparseVector, Value, Vector};

    pub fn literal(value: impl Into<Value>) -> Value {
        value.into()
    }

    pub fn f32_vector(values: Vec<f32>) -> Vector {
        Vector::f32(values)
    }

    pub fn u8_vector(values: Vec<u8>) -> Vector {
        Vector::u8(values)
    }

    pub fn binary_vector(values: Vec<u8>) -> Vector {
        Vector::u8(values)
    }

    pub fn f32_sparse_vector(indices: Vec<u32>, values: Vec<f32>) -> SparseVector {
        SparseVector::f32(indices, values)
    }

    pub fn u8_sparse_vector(indices: Vec<u32>, values: Vec<u8>) -> SparseVector {
        SparseVector::u8(indices, values)
    }
}

pub mod query {
    use crate::proto::v1::data::{
        stage::{filter_stage::FilterExpr, select_stage::SelectExpr},
        text_expr::Term,
        LogicalExpr, Query, Stage, TextExpr,
    };

    pub mod fns {
        use crate::proto::v1::data::{FunctionExpr, QueryVector};

        pub fn vector_distance(
            field: impl Into<String>,
            query: impl Into<QueryVector>,
        ) -> FunctionExpr {
            FunctionExpr::vector_distance(field, query)
        }

        pub fn semantic_similarity(
            field: impl Into<String>,
            query: impl Into<String>,
        ) -> FunctionExpr {
            FunctionExpr::semantic_similarity(field, query)
        }

        pub fn bm25_score() -> FunctionExpr {
            FunctionExpr::bm25_score()
        }
    }

    pub fn field(name: impl Into<String>) -> LogicalExpr {
        LogicalExpr::field(name)
    }

    pub fn select(
        exprs: impl IntoIterator<Item = (impl Into<String>, impl Into<SelectExpr>)>,
    ) -> Query {
        Query::new(vec![Stage::select(exprs)])
    }

    pub fn filter(expr: impl Into<FilterExpr>) -> Query {
        Query::new(vec![Stage::filter(expr.into())])
    }

    pub fn not(expr: impl Into<LogicalExpr>) -> LogicalExpr {
        LogicalExpr::not(expr)
    }

    pub fn r#match(token: &str, field: Option<&str>, weight: Option<f32>, all: bool) -> TextExpr {
        TextExpr::terms(
            all,
            vec![Term {
                token: token.to_string(),
                field: field.map(|s| s.to_string()),
                weight: weight.unwrap_or(1.0),
            }],
        )
    }
}
