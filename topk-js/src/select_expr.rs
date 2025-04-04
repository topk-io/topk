use std::collections::HashMap;

use crate::function_expr::FunctionExpression;
use crate::logical_expr::LogicalExpression;
use crate::query::{Query, Stage};
use napi::{bindgen_prelude::*, NapiValue};
use napi_derive::napi;
use topk_protos::v1::data;

#[derive(Debug, Clone)]
pub enum SelectExpression {
  Logical { expr: LogicalExpression },
  Function { expr: FunctionExpression },
}

// #[napi]
// #[derive(Debug, Clone)]
// pub struct NapiSelectExpression {
//   pub expr: SelectExpression,
// }

// #[napi]
// #[derive(Debug, Clone)]
// pub struct SelectExpression {
//   expr: SelectExpressionUnion,
// }

// #[napi]
// impl SelectExpression {
// pub fn create(expr: SelectExpressionUnion) -> Self {
//   Self { expr }
// }

// #[napi(factory)]
// pub fn select(expr: LogicalExpression) -> Self {
//   Self {
//     expr: SelectExpressionUnion::Logical { expr },
//   }
// }

// // TODO
// #[napi(getter)]
// pub fn get_expr(&self) -> napi::Result<String> {
//   match &self.expr {
//     SelectExpressionUnion::Logical { expr } => Ok(format!("Logical: {:?}", expr)),
//     SelectExpressionUnion::Function { expr } => Ok(format!("Function: {:?}", expr)),
//   }
// }

// #[napi]
// pub fn add(&self, value: f64) -> Self {
//   match &self.expr {
//     SelectExpressionUnion::Logical { expr } => Self {
//       expr: SelectExpressionUnion::Logical {
//         expr: expr.add(&LogicalExpression::Literal {
//           value: Value::F64(value),
//         }),
//       },
//     },
//     _ => unreachable!("add can only be called on logical expressions"),
//   }
// }

// #[napi]
// pub fn subtract(&self, value: f64) -> Self {
//   match &self.expr {
//     SelectExpressionUnion::Logical { expr } => Self {
//       expr: SelectExpressionUnion::Logical {
//         expr: expr.sub(&LogicalExpression::Literal {
//           value: Value::F64(value),
//         }),
//       },
//     },
//     _ => unreachable!("subtract can only be called on logical expressions"),
//   }
// }

// #[napi]
// pub fn mul(&self, value: f64) -> Self {
//   match &self.expr {
//     SelectExpressionUnion::Logical { expr } => Self {
//       expr: SelectExpressionUnion::Logical {
//         expr: expr.mul(&LogicalExpression::Literal {
//           value: Value::F64(value),
//         }),
//       },
//     },
//     _ => unreachable!("multiply can only be called on logical expressions"),
//   }
// }

// #[napi]
// pub fn div(&self, value: f64) -> Self {
//   match &self.expr {
//     SelectExpressionUnion::Logical { expr } => Self {
//       expr: SelectExpressionUnion::Logical {
//         expr: expr.div(&LogicalExpression::Literal {
//           value: Value::F64(value),
//         }),
//       },
//     },
//     _ => unreachable!("divide can only be called on logical expressions"),
//   }
// }
// }

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

impl FromNapiValue for SelectExpression {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    let env_value = Unknown::from_napi_value(env, value)?;
    let env_env = Env::from_raw(env);

    let is_logical_expression = LogicalExpression::instance_of(env_env, env_value)?;

    if (is_logical_expression) {
      Ok(SelectExpression::Logical {
        expr: LogicalExpression::from_napi_value(env, value)?,
      })
    } else {
      Ok(SelectExpression::Function {
        expr: FunctionExpression::from_napi_value(env, value)?,
      })
    }
  }
}

impl ToNapiValue for SelectExpression {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    match val {
      SelectExpression::Logical { expr } => ToNapiValue::to_napi_value(env, expr),
      SelectExpression::Function { expr } => ToNapiValue::to_napi_value(env, expr),
    }
  }
}

#[napi]
pub fn select(
  #[napi(ts_arg_type = "Record<string, LogicalExpression | FunctionExpression>")] exprs: HashMap<
    String,
    SelectExpression,
  >,
) -> Result<Query> {
  let stage = Stage::Select {
    exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
  };

  let stages = vec![stage];

  Ok(Query::create(stages))
}
