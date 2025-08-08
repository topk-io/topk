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

pub enum StringyWithList {
    Stringy(Stringy),
    List(Vec<String>),
}

impl FromNapiValue for StringyWithList {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        if let Ok(list) = Vec::<String>::from_napi_value(env, value) {
            return Ok(StringyWithList::List(list));
        }

        Ok(StringyWithList::Stringy(Stringy::from_napi_value(
            env, value,
        )?))
    }
}

impl Into<LogicalExpression> for StringyWithList {
    fn into(self) -> LogicalExpression {
        match self {
            StringyWithList::Stringy(expr) => expr.into(),
            StringyWithList::List(values) => LogicalExpression::literal(values),
        }
    }
}
