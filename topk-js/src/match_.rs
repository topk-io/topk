// use crate::{logical_expr::LogicalExpression, text_expr::TextExpression};
// use napi::bindgen_prelude::*;
// use napi_derive::napi;

// #[napi]
// #[derive(Debug, Clone)]
// pub struct Match {
//   expr: TextExpression,
// }

// #[napi]
// impl Match {
//   #[napi(factory)]
//   pub fn create(field: String) -> Self {
//     Self {
//       expr: LogicalExpression::Match { field },
//     }
//   }
// }

// #[napi(js_name = "match")]
// pub fn match_() -> Match {
//   Match {
//     expr: TextExpression::Match {
//       field: String::new(),
//     },
//   }
// }
