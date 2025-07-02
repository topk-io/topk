use crate::expr::{function::FunctionExpression, logical::LogicalExpression};
use napi::bindgen_prelude::*;

#[derive(Debug, Clone)]
pub enum SelectExpression {
    Logical { expr: LogicalExpression },
    Function { expr: FunctionExpression },
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

impl From<SelectExpression> for topk_rs::proto::v1::data::stage::select_stage::SelectExpr {
    fn from(expr: SelectExpression) -> Self {
        match expr {
            SelectExpression::Logical { expr } => {
                topk_rs::proto::v1::data::stage::select_stage::SelectExpr::logical(expr)
            }
            SelectExpression::Function { expr } => {
                topk_rs::proto::v1::data::stage::select_stage::SelectExpr::function(expr)
            }
        }
    }
}
