use super::LogicalExpression;
use crate::data::{is_napi_integer, Scalar};
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

        let mut result: i32 = 0;
        check_status!(napi::sys::napi_typeof(env, value, &mut result))?;
        match result {
            napi::sys::ValueType::napi_undefined | napi::sys::ValueType::napi_null => Ok(Comparable::Null(Null{})),
            napi::sys::ValueType::napi_string => {
                Ok(Comparable::String(String::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_number => match is_napi_integer(env, value) {
                true => Ok(Comparable::Int(i64::from_napi_value(env, value)?)),
                false => Ok(Comparable::Float(f64::from_napi_value(env, value)?)),
            },
            napi::sys::ValueType::napi_boolean => {
                Ok(Comparable::Bool(bool::from_napi_value(env, value)?))
            }
            _ => Err(napi::Error::from_reason(
                "Unsupported comparable expression type",
            )),
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
