use crate::{
    data::Scalar,
    expr::{
        filter::FilterExpr,
        logical::{LogicalExpr, UnaryOperator},
        select::SelectExpr,
        text::{Term, TextExpr},
    },
};
use std::collections::HashMap;

mod query;
pub use query::Query;

mod stage;
pub use stage::Stage;

pub fn select<S, E>(exprs: impl IntoIterator<Item = (S, E)>) -> Query
where
    S: Into<String>,
    E: Into<SelectExpr>,
{
    let exprs: HashMap<String, SelectExpr> = exprs
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();

    Query::new(vec![]).select(exprs)
}

pub fn filter(expr: impl Into<FilterExpr>) -> Query {
    Query::new(vec![]).filter(expr.into())
}

pub fn field<S>(name: S) -> LogicalExpr
where
    S: Into<String>,
{
    LogicalExpr::Field { name: name.into() }
}

pub fn literal<V>(value: V) -> LogicalExpr
where
    V: Into<Scalar>,
{
    LogicalExpr::Literal {
        value: value.into(),
    }
}

pub fn r#match(value: &str, field: Option<&str>, weight: Option<f32>) -> TextExpr {
    TextExpr::Terms {
        all: false,
        terms: vec![Term {
            token: value.to_string(),
            field: field.map(|f| f.to_string()),
            weight: weight.unwrap_or(1.0),
        }],
    }
}

pub fn not(expr: impl Into<LogicalExpr>) -> LogicalExpr {
    LogicalExpr::Unary {
        op: UnaryOperator::Not,
        expr: Box::new(expr.into()),
    }
}

pub mod fns {
    use crate::data::Vector;
    use crate::expr::function::FunctionExpr;

    pub fn bm25_score() -> FunctionExpr {
        FunctionExpr::KeywordScore {}
    }

    pub fn vector_distance(field: &str, query: impl Into<Vector>) -> FunctionExpr {
        FunctionExpr::VectorScore {
            field: field.to_string(),
            query: query.into(),
        }
    }

    pub fn semantic_similarity(field: &str, query: &str) -> FunctionExpr {
        FunctionExpr::SemanticSimilarity {
            field: field.to_string(),
            query: query.to_string(),
        }
    }
}
