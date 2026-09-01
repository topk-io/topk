use super::{logical::LogicalExpression, text::TextExpression};
use napi::bindgen_prelude::*;

#[derive(Debug, Clone)]
pub enum FilterExpression {
    Logical { expr: LogicalExpression },
    Text { expr: TextExpression },
}

impl FromNapiValue for FilterExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        if let Ok(expr) = crate::try_cast_ref!(env, value, TextExpression) {
            return Ok(FilterExpression::Text { expr: expr.clone() });
        }

        let expr = unsafe { LogicalExpression::from_napi_value(env, value) }
            .map_err(|_| napi::Error::from_reason("Unsupported filter expression value"))?;
        Ok(FilterExpression::Logical { expr })
    }
}

impl From<FilterExpression> for topk_rs::proto::v1::data::stage::filter_stage::FilterExpr {
    fn from(expr: FilterExpression) -> Self {
        match expr {
            FilterExpression::Logical { expr } => {
                topk_rs::proto::v1::data::stage::filter_stage::FilterExpr::logical(expr)
            }
            FilterExpression::Text { expr } => {
                topk_rs::proto::v1::data::stage::filter_stage::FilterExpr::text(expr)
            }
        }
    }
}
