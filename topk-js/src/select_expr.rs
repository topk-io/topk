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

impl Into<topk_protos::v1::data::LogicalExpr> for LogicalExpression {
  fn into(self) -> topk_protos::v1::data::LogicalExpr {
    match self {
      LogicalExpression::Null => unreachable!(),
      LogicalExpression::Field { name } => topk_protos::v1::data::LogicalExpr::field(name),
      LogicalExpression::Literal { value } => {
        topk_protos::v1::data::LogicalExpr::literal(value.into())
      }
      LogicalExpression::Unary { op, expr } => {
        topk_protos::v1::data::LogicalExpr::unary(op.into(), expr.as_ref().clone().into())
      }
      LogicalExpression::Binary { left, op, right } => topk_protos::v1::data::LogicalExpr::binary(
        op.into(),
        left.as_ref().clone().into(),
        right.as_ref().clone().into(),
      ),
    }
  }
}
