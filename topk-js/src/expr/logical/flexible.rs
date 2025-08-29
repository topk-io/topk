use napi::bindgen_prelude::*;

use crate::{
    data::{List, Scalar, Value, Values},
    expr::logical::LogicalExpression,
};

#[derive(Debug, Clone)]
pub enum FlexibleExpression {
    String(String),
    Int(i64),
    Float(f64),
    Expr(LogicalExpression),
}

impl FromNapiValue for FlexibleExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(FlexibleExpression::Expr(expr.clone()));
        }

        match Value::from_napi_value(env, value)? {
            Value::String(s) => Ok(FlexibleExpression::String(s)),
            Value::I64(i) => Ok(FlexibleExpression::Int(i)),
            Value::F64(f) => Ok(FlexibleExpression::Float(f)),
            v => Err(napi::Error::from_reason(format!(
                "Unsupported flexible expression type: {:?}",
                v
            ))),
        }
    }
}

impl Into<LogicalExpression> for FlexibleExpression {
    fn into(self) -> LogicalExpression {
        match self {
            FlexibleExpression::String(s) => LogicalExpression::literal(Scalar::String(s)),
            FlexibleExpression::Int(i) => LogicalExpression::literal(Scalar::I64(i)),
            FlexibleExpression::Float(f) => LogicalExpression::literal(Scalar::F64(f)),
            FlexibleExpression::Expr(e) => e,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Iterable {
    String(String),
    List(List),
    StringList(Vec<String>),
    IntList(Vec<i64>),
    FloatList(Vec<f32>),
    Expr(LogicalExpression),
}

impl FromNapiValue for Iterable {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(Iterable::Expr(expr.clone()));
        }

        match Value::from_napi_value(env, value)? {
            Value::String(s) => Ok(Iterable::String(s)),
            Value::List(l) => Ok(Iterable::List(l)),
            _ => Err(napi::Error::from_reason(format!(
                "Unsupported iterable expression type: {:?}",
                value
            ))),
        }
    }
}

impl Into<LogicalExpression> for Iterable {
    fn into(self) -> LogicalExpression {
        match self {
            Iterable::String(s) => LogicalExpression::literal(Scalar::String(s)),
            Iterable::List(l) => LogicalExpression::literal(Scalar::List(l)),
            Iterable::StringList(l) => LogicalExpression::literal(Scalar::List(List {
                values: Values::String(l),
            })),
            Iterable::IntList(l) => LogicalExpression::literal(Scalar::List(List {
                values: Values::I64(l),
            })),
            Iterable::FloatList(l) => LogicalExpression::literal(Scalar::List(List {
                values: Values::F32(l),
            })),
            Iterable::Expr(e) => e,
        }
    }
}
