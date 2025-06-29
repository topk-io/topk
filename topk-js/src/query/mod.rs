pub mod query;
pub mod stage;

use std::collections::HashMap;

use crate::{
    data::{scalar::Scalar, value::Value},
    expr::{
        filter::FilterExpression,
        function::{FunctionExpression, QueryVector},
        logical::{LogicalExpression, UnaryOperator},
        select::SelectExpression,
    },
    query::{query::Query, stage::Stage},
};
use napi_derive::napi;

#[napi(namespace = "query")]
pub fn select(
    #[napi(ts_arg_type = "Record<string, LogicalExpression | FunctionExpression>")] exprs: HashMap<
        String,
        SelectExpression,
    >,
) -> Query {
    Query {
        stages: vec![Stage::Select {
            exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }],
    }
}

#[napi(namespace = "query")]
pub fn filter(
    #[napi(ts_arg_type = "LogicalExpression | TextExpression")] expr: FilterExpression,
) -> Query {
    Query {
        stages: vec![Stage::Filter { expr }],
    }
}

#[napi(namespace = "query")]
pub fn field(name: String) -> LogicalExpression {
    LogicalExpression::field(name)
}

#[napi(namespace = "query")]
pub fn literal(
    #[napi(ts_arg_type = "number | string | boolean")] value: Value,
) -> napi::Result<LogicalExpression> {
    match value {
        Value::String(s) => Ok(LogicalExpression::literal(Scalar::String(s))),
        Value::Bool(b) => Ok(LogicalExpression::literal(Scalar::Bool(b))),
        Value::I64(i) => Ok(LogicalExpression::literal(Scalar::I64(i))),
        Value::F64(f) => Ok(LogicalExpression::literal(Scalar::F64(f))),
        v => Err(napi::Error::from_reason(format!(
            "Unsupported scalar type: {:?}",
            v
        ))),
    }
}

#[napi(js_name = "not", namespace = "query")]
pub fn not(expr: &'static LogicalExpression) -> LogicalExpression {
    LogicalExpression::unary(UnaryOperator::Not, expr.clone())
}

#[napi(namespace = "query")]
pub fn vector_distance(
    field: String,
    #[napi(
        ts_arg_type = "Array<number> | Record<number, number> | data.Vector | data.SparseVector"
    )]
    query: Value,
) -> napi::Result<FunctionExpression> {
    match query {
        Value::Vector(query) => Ok(FunctionExpression::vector_score(
            field,
            QueryVector::Dense { query },
        )),
        Value::SparseVector(query) => Ok(FunctionExpression::vector_score(
            field,
            QueryVector::Sparse { query },
        )),
        v => Err(napi::Error::from_reason(format!("Unsupported vector query: {:?}", v)).into()),
    }
}

#[napi(namespace = "query")]
pub fn bm25_score() -> FunctionExpression {
    FunctionExpression::keyword_score()
}

#[napi(namespace = "query")]
pub fn semantic_similarity(field: String, query: String) -> FunctionExpression {
    FunctionExpression::semantic_similarity(field, query)
}
