pub mod r#fn;
pub mod query;
pub mod stage;

use crate::{
    data::{Scalar, Value},
    expr::{
        filter::FilterExpression,
        logical::{BinaryOperator, LogicalExpression, NaryOp, Numeric, UnaryOperator},
        select::SelectExpression,
    },
    query::{query::Query, stage::Stage},
};
use napi_derive::napi;
use std::collections::HashMap;

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
    #[napi(ts_arg_type = "number | string | string[] | number[] | boolean | data.List")]
    value: Value,
) -> napi::Result<LogicalExpression> {
    match value {
        Value::String(s) => Ok(LogicalExpression::literal(Scalar::String(s))),
        Value::Bool(b) => Ok(LogicalExpression::literal(Scalar::Bool(b))),
        Value::I64(i) => Ok(LogicalExpression::literal(Scalar::I64(i))),
        Value::F64(f) => Ok(LogicalExpression::literal(Scalar::F64(f))),
        Value::List(l) => Ok(LogicalExpression::literal(Scalar::List(l))),
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

/// Evaluates to true if each `expr` is true.
#[napi(js_name = "all", namespace = "query")]
pub fn all(exprs: Vec<&'static LogicalExpression>) -> LogicalExpression {
    LogicalExpression::nary(NaryOp::All, exprs.into_iter().map(|e| e.clone()).collect())
}

/// Evaluates to true if at least one `expr` is true.
#[napi(js_name = "any", namespace = "query")]
pub fn any(exprs: Vec<&'static LogicalExpression>) -> LogicalExpression {
    LogicalExpression::nary(NaryOp::Any, exprs.into_iter().map(|e| e.clone()).collect())
}

#[napi(js_name = "min", namespace = "query")]
pub fn min(
    #[napi(ts_arg_type = "LogicalExpression | number")] left: Numeric,
    #[napi(ts_arg_type = "LogicalExpression | number")] right: Numeric,
) -> LogicalExpression {
    LogicalExpression::binary(BinaryOperator::Min, left.into(), right.into())
}

#[napi(js_name = "max", namespace = "query")]
pub fn max(
    #[napi(ts_arg_type = "LogicalExpression | number")] left: Numeric,
    #[napi(ts_arg_type = "LogicalExpression | number")] right: Numeric,
) -> LogicalExpression {
    LogicalExpression::binary(BinaryOperator::Max, left.into(), right.into())
}

#[napi(js_name = "abs", namespace = "query")]
pub fn abs(expr: &'static LogicalExpression) -> LogicalExpression {
    LogicalExpression::unary(UnaryOperator::Abs, expr.clone())
}
