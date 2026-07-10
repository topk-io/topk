use napi::bindgen_prelude::*;
use napi_derive::napi;

/// @internal
/// @hideconstructor
#[napi(namespace = "query")]
#[derive(Debug, Clone)]
pub struct AggregateExpression(pub(crate) AggregateExpressionUnion);

impl FromNapiValue for AggregateExpression {
    unsafe fn from_napi_value(
        env: napi::sys::napi_env,
        value: napi::sys::napi_value,
    ) -> napi::Result<Self> {
        let expr = crate::try_cast_ref!(env, value, AggregateExpression)?;
        Ok(expr.clone())
    }
}

#[derive(Debug, Clone)]
pub enum AggregateExpressionUnion {
    Count { field: Option<String> },
    Sum { field: String },
    Min { field: String },
    Max { field: String },
    Avg { field: String },
}

impl From<AggregateExpression> for topk_rs::proto::v1::data::AggregateExpr {
    fn from(expr: AggregateExpression) -> Self {
        match expr.0 {
            AggregateExpressionUnion::Count { field } => {
                topk_rs::proto::v1::data::AggregateExpr::count(field)
            }
            AggregateExpressionUnion::Sum { field } => {
                topk_rs::proto::v1::data::AggregateExpr::sum(field)
            }
            AggregateExpressionUnion::Min { field } => {
                topk_rs::proto::v1::data::AggregateExpr::min(field)
            }
            AggregateExpressionUnion::Max { field } => {
                topk_rs::proto::v1::data::AggregateExpr::max(field)
            }
            AggregateExpressionUnion::Avg { field } => {
                topk_rs::proto::v1::data::AggregateExpr::avg(field)
            }
        }
    }
}
