use super::LogicalExpression;
use crate::data::{Scalar, Value};
use napi::bindgen_prelude::*;

#[derive(Debug, Clone)]
pub enum Comparable {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null(Null),
    Expr(LogicalExpression),
}

impl FromNapiValue for Comparable {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(Comparable::Expr(expr.clone()));
        }

        match Value::from_napi_value(env, value)? {
            Value::String(s) => Ok(Comparable::String(s)),
            Value::I64(i) => Ok(Comparable::Int(i)),
            Value::F64(f) => Ok(Comparable::Float(f)),
            Value::Bool(b) => Ok(Comparable::Bool(b)),
            Value::Null => Ok(Comparable::Null(Null {})),
            v => Err(napi::Error::from_reason(format!(
                "Unsupported comparable expression type: {:?}",
                v
            ))),
        }
    }
}

impl Into<LogicalExpression> for Comparable {
    fn into(self) -> LogicalExpression {
        match self {
            Comparable::String(s) => LogicalExpression::literal(Scalar::String(s)),
            Comparable::Int(i) => LogicalExpression::literal(Scalar::I64(i)),
            Comparable::Float(f) => LogicalExpression::literal(Scalar::F64(f)),
            Comparable::Bool(b) => LogicalExpression::literal(Scalar::Bool(b)),
            Comparable::Null(_) => LogicalExpression::null(),
            Comparable::Expr(e) => e,
        }
    }
}
