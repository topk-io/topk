use super::{logical::LogicalExpression, text::TextExpression};
use napi::bindgen_prelude::*;

#[derive(Debug, Clone)]
pub struct FilterExpression {
    expr: FilterExpressionUnion,
}

#[derive(Debug, Clone)]
pub enum FilterExpressionUnion {
    Logical { expr: LogicalExpression },
    Text { expr: TextExpression },
}

impl FromNapiValue for FilterExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, LogicalExpression) {
            return Ok(FilterExpression {
                expr: FilterExpressionUnion::Logical { expr: expr.clone() },
            });
        }

        if let Ok(expr) = crate::try_cast_ref!(env, value, TextExpression) {
            return Ok(FilterExpression {
                expr: FilterExpressionUnion::Text { expr: expr.clone() },
            });
        }

        Err(napi::Error::from_reason(
            "Unsupported filter expression value",
        ))
    }
}

impl Into<topk_rs::expr::filter::FilterExpr> for FilterExpression {
    fn into(self) -> topk_rs::expr::filter::FilterExpr {
        match self.expr {
            FilterExpressionUnion::Logical { expr } => {
                topk_rs::expr::filter::FilterExpr::Logical(expr.into())
            }
            FilterExpressionUnion::Text { expr } => {
                topk_rs::expr::filter::FilterExpr::Text(expr.into())
            }
        }
    }
}
