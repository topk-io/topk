use napi::bindgen_prelude::*;

use super::{
    logical_expr::{LogicalExpression, LogicalExpressionUnion},
    scalar::Scalar,
    utils::is_napi_integer,
};

#[derive(Debug, Clone)]
pub enum FlexibleExpr {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null(Null),
    Expr(LogicalExpression),
}

impl FromNapiValue for FlexibleExpr {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let env_env = Env::from_raw(env);

        let is_logical_expression = {
            let env_value = Unknown::from_napi_value(env, value)?;
            LogicalExpression::instance_of(env_env, env_value)?
        };

        if is_logical_expression {
            return Ok(FlexibleExpr::Expr(LogicalExpression::from_napi_value(
                env, value,
            )?));
        }

        let mut result: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut result);

        match result {
            napi::sys::ValueType::napi_string => {
                Ok(FlexibleExpr::String(String::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_number => match is_napi_integer(env, value) {
                true => Ok(FlexibleExpr::Int(i64::from_napi_value(env, value)?)),
                false => Ok(FlexibleExpr::Float(f64::from_napi_value(env, value)?)),
            },
            napi::sys::ValueType::napi_boolean => {
                Ok(FlexibleExpr::Bool(bool::from_napi_value(env, value)?))
            }
            napi::sys::ValueType::napi_null => Ok(FlexibleExpr::Null(Null)),
            napi::sys::ValueType::napi_undefined => Ok(FlexibleExpr::Null(Null)),
            _ => Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("Unsupported flexible expression type: {}", result),
            )),
        }
    }
}

impl ToNapiValue for FlexibleExpr {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        match val {
            FlexibleExpr::String(s) => String::to_napi_value(env, s),
            FlexibleExpr::Int(i) => i64::to_napi_value(env, i),
            FlexibleExpr::Float(f) => f64::to_napi_value(env, f),
            FlexibleExpr::Bool(b) => bool::to_napi_value(env, b),
            FlexibleExpr::Null(_) => Null::to_napi_value(env, Null),
            FlexibleExpr::Expr(e) => LogicalExpression::to_napi_value(env, e),
        }
    }
}

impl Into<LogicalExpressionUnion> for FlexibleExpr {
    fn into(self) -> LogicalExpressionUnion {
        match self {
            FlexibleExpr::String(s) => LogicalExpressionUnion::Literal {
                value: Scalar::String(s),
            },
            FlexibleExpr::Int(i) => LogicalExpressionUnion::Literal {
                value: Scalar::I64(i),
            },
            FlexibleExpr::Float(f) => LogicalExpressionUnion::Literal {
                value: Scalar::F64(f),
            },
            FlexibleExpr::Bool(b) => LogicalExpressionUnion::Literal {
                value: Scalar::Bool(b),
            },
            FlexibleExpr::Null(_) => LogicalExpressionUnion::Null,
            FlexibleExpr::Expr(e) => e.get_expr(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Numeric {
    Int(i64),
    Float(f64),
    Expr(LogicalExpression),
}

impl FromNapiValue for Numeric {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let env_env = Env::from_raw(env);

        let is_logical_expression = {
            let env_value = Unknown::from_napi_value(env, value)?;
            LogicalExpression::instance_of(env_env, env_value)?
        };

        if is_logical_expression {
            return Ok(Numeric::Expr(LogicalExpression::from_napi_value(
                env, value,
            )?));
        }

        let mut result: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut result);

        match result {
            napi::sys::ValueType::napi_number => match is_napi_integer(env, value) {
                true => Ok(Numeric::Int(i64::from_napi_value(env, value)?)),
                false => Ok(Numeric::Float(f64::from_napi_value(env, value)?)),
            },
            _ => Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("Unsupported numeric type: {}", result),
            )),
        }
    }
}

impl ToNapiValue for Numeric {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        match val {
            Numeric::Int(i) => i64::to_napi_value(env, i),
            Numeric::Float(f) => f64::to_napi_value(env, f),
            Numeric::Expr(e) => LogicalExpression::to_napi_value(env, e),
        }
    }
}

impl Into<LogicalExpressionUnion> for Numeric {
    fn into(self) -> LogicalExpressionUnion {
        match self {
            Numeric::Int(i) => LogicalExpressionUnion::Literal {
                value: Scalar::I64(i),
            },
            Numeric::Float(f) => LogicalExpressionUnion::Literal {
                value: Scalar::F64(f),
            },
            Numeric::Expr(e) => e.get_expr(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Boolish {
    Bool(bool),
    Expr(LogicalExpression),
}

impl FromNapiValue for Boolish {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let env_env = Env::from_raw(env);

        let is_logical_expression = {
            let env_value = Unknown::from_napi_value(env, value)?;
            LogicalExpression::instance_of(env_env, env_value)?
        };

        if is_logical_expression {
            return Ok(Boolish::Expr(LogicalExpression::from_napi_value(
                env, value,
            )?));
        }

        let mut result: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut result);

        match result {
            napi::sys::ValueType::napi_boolean => {
                Ok(Boolish::Bool(bool::from_napi_value(env, value)?))
            }
            _ => Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("Unsupported boolish type: {}", result),
            )),
        }
    }
}

impl ToNapiValue for Boolish {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        match val {
            Boolish::Bool(b) => bool::to_napi_value(env, b),
            Boolish::Expr(e) => LogicalExpression::to_napi_value(env, e),
        }
    }
}

impl Into<LogicalExpressionUnion> for Boolish {
    fn into(self) -> LogicalExpressionUnion {
        match self {
            Boolish::Bool(b) => LogicalExpressionUnion::Literal {
                value: Scalar::Bool(b),
            },
            Boolish::Expr(e) => e.get_expr(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stringy {
    String(String),
    Expr(LogicalExpression),
}

impl FromNapiValue for Stringy {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> Result<Self, napi::Status> {
        let env_env = Env::from_raw(env);

        let is_logical_expression = {
            let env_value = Unknown::from_napi_value(env, value)?;
            LogicalExpression::instance_of(env_env, env_value)?
        };

        if is_logical_expression {
            return Ok(Stringy::Expr(LogicalExpression::from_napi_value(
                env, value,
            )?));
        }

        let mut result: i32 = 0;
        napi::sys::napi_typeof(env, value, &mut result);

        match result {
            napi::sys::ValueType::napi_string => {
                Ok(Stringy::String(String::from_napi_value(env, value)?))
            }
            _ => unreachable!("Unsupported stringy type: {}", result),
        }
    }
}

impl ToNapiValue for Stringy {
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        val: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        match val {
            Stringy::String(s) => String::to_napi_value(env, s),
            Stringy::Expr(e) => LogicalExpression::to_napi_value(env, e),
        }
    }
}

impl Into<LogicalExpressionUnion> for Stringy {
    fn into(self) -> LogicalExpressionUnion {
        match self {
            Stringy::String(s) => LogicalExpressionUnion::Literal {
                value: Scalar::String(s),
            },
            Stringy::Expr(e) => e.get_expr(),
        }
    }
}
