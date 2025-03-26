use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::my_box::MyBox;

#[napi]
#[derive(Debug, Clone)]
pub enum LogicalExpression {
  Null,
  Field {
    name: String,
  },
  Literal {
    value: String,
  },
  And {
    #[napi(ts_type = "LogicalExpression")]
    left: MyBox<LogicalExpression>,
    #[napi(ts_type = "LogicalExpression")]
    right: MyBox<LogicalExpression>,
  },
  Or {
    #[napi(ts_type = "LogicalExpression")]
    left: MyBox<LogicalExpression>,
    #[napi(ts_type = "LogicalExpression")]
    right: MyBox<LogicalExpression>,
  },
}
