use crate::function_expr::FunctionExpression;
use crate::logical_expr::LogicalExpression;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

#[napi]
#[derive(Debug, Clone)]
pub enum SelectExpression {
  Logical { expr: LogicalExpression },
  Function { expr: FunctionExpression },
}

impl Into<data::stage::select_stage::SelectExpr> for SelectExpression {
  fn into(self) -> data::stage::select_stage::SelectExpr {
    match self {
      SelectExpression::Logical { expr } => {
        data::stage::select_stage::SelectExpr::logical(expr.into())
      }
      SelectExpression::Function { expr } => {
        data::stage::select_stage::SelectExpr::function(expr.into())
      }
    }
  }
}

impl From<LogicalExpression> for data::LogicalExpr {
  fn from(expr: LogicalExpression) -> Self {
    match expr {
      LogicalExpression::And { left, right } => {
        todo!()
      }
      _ => {
        todo!()
      }
    }
  }
}
