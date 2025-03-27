use super::{logical_expr::LogicalExpression, text_expr::TextExpression};
use napi_derive::napi;
use topk_protos::v1::data;

#[napi]
#[derive(Debug, Clone)]
pub enum FilterExpression {
  Logical { expr: LogicalExpression },
  Text { expr: TextExpression },
}

impl Into<data::stage::filter_stage::FilterExpr> for FilterExpression {
  fn into(self) -> data::stage::filter_stage::FilterExpr {
    match self {
      FilterExpression::Logical { expr } => {
        data::stage::filter_stage::FilterExpr::logical(expr.into())
      }
      FilterExpression::Text { expr } => data::stage::filter_stage::FilterExpr::text(expr.into()),
    }
  }
}
