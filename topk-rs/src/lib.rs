pub mod proto;

pub mod error;
pub use error::Error;

pub mod client;
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
    use crate::proto::v1::data::{SparseVector, Value};

    pub fn literal(value: impl Into<Value>) -> Value {
        value.into()
    }

    pub fn list<T: crate::proto::v1::data::IntoListValues>(values: T) -> Value {
        Value::list(values)
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
        use crate::proto::v1::data::{FunctionExpr, Value};

        pub fn vector_distance(field: impl Into<String>, query: impl Into<Value>) -> FunctionExpr {
            FunctionExpr::vector_distance(field, query, false)
        }

        pub fn multi_vector_distance(
            field: impl Into<String>,
            query: impl Into<Value>,
            candidates: Option<u32>,
        ) -> FunctionExpr {
            FunctionExpr::multi_vector_distance(field, query, candidates)
        }

        pub fn semantic_similarity(
            field: impl Into<String>,
            query: impl Into<String>,
        ) -> FunctionExpr {
            FunctionExpr::semantic_similarity(field, query)
        }

        pub fn bm25_score(b: Option<f32>, k1: Option<f32>) -> FunctionExpr {
            FunctionExpr::bm25_score(b, k1)
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

    /// Evaluates to true if each `expr` is true.
    pub fn all(exprs: impl IntoIterator<Item = impl Into<LogicalExpr>>) -> LogicalExpr {
        LogicalExpr::all(exprs)
    }

    /// Evaluates to true if at least one `expr` is true.
    pub fn any(exprs: impl IntoIterator<Item = impl Into<LogicalExpr>>) -> LogicalExpr {
        LogicalExpr::any(exprs)
    }

    /// Filters documents that match the text.
    pub fn r#match(
        text: impl Into<String>,
        field: Option<&str>,
        weight: Option<f32>,
        all: bool,
    ) -> TextExpr {
        TextExpr::terms(
            all,
            vec![Term {
                token: text.into(),
                field: field.map(|s| s.to_string()),
                weight: weight.unwrap_or(1.0),
            }],
        )
    }

    pub trait AsTerm {
        fn as_term(self) -> (String, f32);
    }

    impl<T: Into<String>> AsTerm for (T, f32) {
        fn as_term(self) -> (String, f32) {
            (self.0.into(), self.1)
        }
    }

    impl AsTerm for String {
        fn as_term(self) -> (String, f32) {
            (self, 1.0)
        }
    }

    impl AsTerm for &str {
        fn as_term(self) -> (String, f32) {
            (self.to_string(), 1.0)
        }
    }

    /// Filters documents that match the provided tokens.
    pub fn match_tokens(
        tokens: impl IntoIterator<Item = impl AsTerm>,
        field: Option<&str>,
        all: bool,
    ) -> TextExpr {
        TextExpr::terms(
            all,
            tokens
                .into_iter()
                .map(|token| {
                    let (token, weight) = token.as_term();
                    Term {
                        token,
                        weight,
                        field: field.map(|s| s.to_string()),
                    }
                })
                .collect(),
        )
    }
}
