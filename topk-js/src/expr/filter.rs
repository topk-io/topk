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

impl From<FilterExpression> for topk_rs::proto::v1::data::stage::filter_stage::FilterExpr {
    fn from(expr: FilterExpression) -> Self {
        match expr.expr {
            FilterExpressionUnion::Logical { expr } => {
                topk_rs::proto::v1::data::stage::filter_stage::FilterExpr::logical(expr)
            }
            FilterExpressionUnion::Text { expr } => {
                topk_rs::proto::v1::data::stage::filter_stage::FilterExpr::text(expr)
            }
        }
    }
}
