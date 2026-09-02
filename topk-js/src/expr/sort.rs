use super::{function::FunctionExpression, logical::LogicalExpression};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Sort order.
#[napi(string_enum = "lowercase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl From<bool> for SortOrder {
    fn from(asc: bool) -> Self {
        if asc {
            SortOrder::Asc
        } else {
            SortOrder::Desc
        }
    }
}

impl From<SortOrder> for topk_rs::proto::v1::data::stage::sort_stage::SortOrder {
    fn from(order: SortOrder) -> Self {
        match order {
            SortOrder::Asc => Self::Asc,
            SortOrder::Desc => Self::Desc,
        }
    }
}

/// An expression to sort by with its sort order.
#[napi(object, namespace = "query")]
#[derive(Debug, Clone)]
pub struct SortExpr {
    /// The expression to sort by.
    #[napi(ts_type = "LogicalExpression | FunctionExpression")]
    pub expr: LogicalExpression,
    /// Sort order.
    pub order: SortOrder,
}

/// Either a single expression to sort by, or an array of `(expr, order)` pairs.
pub enum SortArg {
    Single(LogicalExpression),
    Many(Vec<SortExpr>),
}

impl FromNapiValue for SortArg {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(exprs) = Vec::<SortExpr>::from_napi_value(env, value) {
            return Ok(SortArg::Many(exprs));
        }

        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(SortArg::Single(expr.clone()));
        }

        let expr = crate::try_cast_ref!(env, value, FunctionExpression)?;
        Ok(SortArg::Single(expr.lifted()))
    }
}
