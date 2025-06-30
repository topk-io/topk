use super::LogicalExpression;
use crate::data::Scalar;
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

        if let Ok(string) = String::from_napi_value(env, value) {
            return Ok(Comparable::String(string));
        }

        if let Ok(int) = i64::from_napi_value(env, value) {
            return Ok(Comparable::Int(int));
        }

        if let Ok(float) = f64::from_napi_value(env, value) {
            return Ok(Comparable::Float(float));
        }

        if let Ok(bool) = bool::from_napi_value(env, value) {
            return Ok(Comparable::Bool(bool));
        }

        if let Ok(null) = Null::from_napi_value(env, value) {
            return Ok(Comparable::Null(null));
        }

        Err(napi::Error::from_reason(
            "Unsupported comparable expression type",
        ))
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
