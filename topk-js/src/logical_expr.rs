use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::{
  binary_expr::BinaryOperator, document::Value, my_box::MyBox, unary_expr::UnaryOperator,
};

#[napi]
#[derive(Debug, Clone)]
pub enum LogicalExpression {
  Null,
  Field {
    name: String,
  },
  Literal {
    value: Value,
  },
  Unary {
    op: UnaryOperator,
    #[napi(ts_type = "LogicalExpression")]
    expr: MyBox<LogicalExpression>,
  },
  Binary {
    #[napi(ts_type = "LogicalExpression")]
    left: MyBox<LogicalExpression>,
    op: BinaryOperator,
    #[napi(ts_type = "LogicalExpression")]
    right: MyBox<LogicalExpression>,
  },
}
