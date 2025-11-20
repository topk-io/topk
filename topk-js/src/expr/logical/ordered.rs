use super::{LogicalExpression, Numeric};
use napi::bindgen_prelude::*;

pub enum Ordered {
    Numeric(Numeric),
    String(String),
}

impl FromNapiValue for Ordered {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(string) = String::from_napi_value(env, value) {
            return Ok(Ordered::String(string));
        }

        if let Ok(numeric) = Numeric::from_napi_value(env, value) {
            return Ok(Ordered::Numeric(numeric));
        }

        Err(napi::Error::from_reason("Unsupported ordered type"))
    }
}

impl Into<LogicalExpression> for Ordered {
    fn into(self) -> LogicalExpression {
        match self {
            Ordered::Numeric(expr) => expr.into(),
            Ordered::String(s) => LogicalExpression::literal(s),
        }
    }
}
