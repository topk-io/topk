use crate::expr::{function::FunctionExpression, logical::LogicalExpression};
use napi::bindgen_prelude::*;

#[derive(Debug, Clone)]
pub enum SelectExpression {
    Logical { expr: LogicalExpression },
    Function { expr: FunctionExpression },
}

impl Into<topk_rs::expr::select::SelectExpr> for SelectExpression {
    fn into(self) -> topk_rs::expr::select::SelectExpr {
        match self {
            SelectExpression::Logical { expr } => {
                topk_rs::expr::select::SelectExpr::Logical(expr.into())
            }
            SelectExpression::Function { expr } => {
                topk_rs::expr::select::SelectExpr::Function(expr.into())
            }
        }
    }
}

impl FromNapiValue for SelectExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(SelectExpression::Logical { expr: expr.clone() });
        }

        if let Ok(expr) = crate::try_cast_ref!(env, value, FunctionExpression) {
            return Ok(SelectExpression::Function { expr: expr.clone() });
        }

        Err(napi::Error::from_reason(
            "Value must be either a LogicalExpression or FunctionExpression",
        ))
    }
}
