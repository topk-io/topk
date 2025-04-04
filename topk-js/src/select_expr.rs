use std::collections::HashMap;

use crate::function_expr::FunctionExpression;
use crate::logical_expr::LogicalExpression;
use crate::query::{Query, Stage};
use napi::{bindgen_prelude::*, NapiValue};
use napi_derive::napi;
use topk_protos::v1::data;

#[napi]
#[derive(Debug, Clone)]
pub enum SelectExpression {
  Logical { expr: LogicalExpression },
  Function { expr: FunctionExpression },
}

#[napi]
#[derive(Debug, Clone)]
pub struct NapiSelectExpression {
  pub expr: SelectExpression,
}

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

// impl FromNapiValue for SelectExpression {
//   unsafe fn from_napi_value(
//     env: napi::sys::napi_env,
//     value: napi::sys::napi_value,
//   ) -> Result<Self, napi::Status> {
//     let object = Object::from_napi_value(env, value)?;

//     // Check if it's a logical expression
//     if let Ok(Some(expr)) = object.get::<LogicalExpression>("logical") {
//       return Ok(SelectExpression::Logical { expr });
//     }

//     // Check if it's a function expression
//     if let Ok(Some(expr)) = object.get::<FunctionExpression>("function") {
//       return Ok(SelectExpression::Function { expr });
//     }

//     Err(napi::Error::new(
//       napi::Status::GenericFailure,
//       "Invalid SelectExpressionUnion: missing 'logical' or 'function' property".to_string(),
//     ))
//   }
// }

impl FromNapiValue for NapiSelectExpression {
  unsafe fn from_napi_value(
    env: napi::sys::napi_env,
    value: napi::sys::napi_value,
  ) -> Result<Self, napi::Status> {
    todo!()
    // let object = Object::from_napi_value(env, value)?;

    // let expr = if let Ok(Some(expr_obj)) = object.get::<Object>("expr") {
    //   // If it has an "expr" property, it's a Logical expression
    //   let expr = LogicalExpression::from_napi_value(env, value)?;
    //   SelectExpression::Logical(expr)
    // } else {
    //   // Otherwise, it's a Function expression
    //   let func_expr = FunctionExpression::from_napi_value(env, value)?;
    //   SelectExpression::Function(func_expr)
    // };

    // Ok(expr)
  }
}

impl ToNapiValue for &mut SelectExpression {
  unsafe fn to_napi_value(
    env: napi::sys::napi_env,
    val: Self,
  ) -> napi::Result<napi::sys::napi_value> {
    todo!()
    // let mut result = std::ptr::null_mut();

    // let obj = unsafe {
    //   napi::bindgen_prelude::Object::from_raw_unchecked(
    //     env,
    //     napi::bindgen_prelude::sys::napi_create_object(env, result),
    //   )
    // };

    // match val {
    //   SelectExpression::Logical { expr } => {
    //     obj.set_named_property("type", "Logical")?;
    //     obj.set_named_property("expr", expr)?;
    //   }
    //   SelectExpression::Function { expr } => {
    //     obj.set_named_property("type", "Function")?;
    //     obj.set_named_property("expr", expr)?;
    //   }
    // }

    // Ok(NapiValue::from_raw(env, result))

    // match val {
    //   SelectExpression::Logical { expr } => ToNapiValue::to_napi_value(env, expr),
    //   SelectExpression::Function { expr } => ToNapiValue::to_napi_value(env, expr),
    // }
  }
}

#[napi]
pub fn select(
  // #[napi(ts_arg_type = "Record<string, LogicalExpression | FunctionExpression>")] exprs: HashMap<
  exprs: HashMap<String, SelectExpression>,
) -> Result<Query> {
  let stage = Stage::Select {
    exprs: exprs.into_iter().map(|(k, v)| (k, v.into())).collect(),
  };

  let stages = vec![stage];

  Ok(Query::create(stages))
}
