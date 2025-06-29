use super::LogicalExpression;
use napi::bindgen_prelude::*;

pub enum Boolish {
    Logical(LogicalExpression),
    Bool(bool),
}

impl Into<LogicalExpression> for Boolish {
    fn into(self) -> LogicalExpression {
        match self {
            Boolish::Bool(value) => LogicalExpression::literal(value),
            Boolish::Logical(expr) => expr,
        }
    }
}

impl FromNapiValue for Boolish {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(Boolish::Logical(expr.clone()));
        }

        if let Ok(bool) = bool::from_napi_value(env, value) {
            return Ok(Boolish::Bool(bool));
        }

        Err(napi::Error::from_reason("Unsupported bool type"))
    }
}
