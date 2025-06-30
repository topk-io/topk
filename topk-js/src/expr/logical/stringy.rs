use super::LogicalExpression;
use napi::bindgen_prelude::*;

pub enum Stringy {
    Logical(LogicalExpression),
    String(String),
}

impl Into<LogicalExpression> for Stringy {
    fn into(self) -> LogicalExpression {
        match self {
            Stringy::Logical(expr) => expr,
            Stringy::String(value) => LogicalExpression::literal(value),
        }
    }
}

impl FromNapiValue for Stringy {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(Stringy::Logical(expr.clone()));
        }

        Ok(Stringy::String(String::from_napi_value(env, value)?))
    }
}
