use super::LogicalExpression;
use crate::data::Value;
use napi::bindgen_prelude::*;

pub enum Numeric {
    I64(i64),
    F64(f64),
    Logical(LogicalExpression),
}

impl Into<LogicalExpression> for Numeric {
    fn into(self) -> LogicalExpression {
        match self {
            Numeric::I64(value) => LogicalExpression::literal(value),
            Numeric::F64(value) => LogicalExpression::literal(value),
            Numeric::Logical(expr) => expr,
        }
    }
}

impl FromNapiValue for Numeric {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(Numeric::Logical(expr.clone()));
        }

        match Value::from_napi_value(env, value)? {
            Value::F64(number) => Ok(Numeric::F64(number)),
            Value::I64(number) => Ok(Numeric::I64(number)),
            v => Err(napi::Error::from_reason(format!("Unsupported numeric type: {:?}", v)).into()),
        }
    }
}
