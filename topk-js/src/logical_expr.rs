use napi::bindgen_prelude::*;
use napi_derive::napi;
use topk_protos::v1::data;

#[napi]
pub enum LogicalExpression {
  Null,
  Field {
    name: String,
  },
  Literal {
    value: String,
  },
  And {
    left: Reference<LogicalExpression>,
    right: Reference<LogicalExpression>,
  },
  //   Or {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Not {
  //     expr: Box<LogicalExpression>,
  //   },
  //   Eq {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   NotEq {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Lt {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   LtEq {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Gt {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   GtEq {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Add {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Sub {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Mul {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   Div {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
  //   StartsWith {
  //     left: Box<LogicalExpression>,
  //     right: Box<LogicalExpression>,
  //   },
}

// impl Into<data::LogicalExpr> for LogicalExpression {
//   fn into(self) -> data::LogicalExpr {
//     match self {
//       LogicalExpression::Null => data::LogicalExpr::null(),
//       LogicalExpression::Field { name } => data::LogicalExpr::field(name),
//       LogicalExpression::Literal { value } => data::LogicalExpr::literal(value.into()),
//       LogicalExpression::And { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::And, left.into(), right.into())
//       }
//       LogicalExpression::Or { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Or, left.into(), right.into())
//       }
//       LogicalExpression::Not { expr } => data::LogicalExpr::unary(data::UnaryOp::Not, expr.into()),
//       LogicalExpression::Eq { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Eq, left.into(), right.into())
//       }
//       LogicalExpression::NotEq { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::NotEq, left.into(), right.into())
//       }
//       LogicalExpression::Lt { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Lt, left.into(), right.into())
//       }
//       LogicalExpression::LtEq { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::LtEq, left.into(), right.into())
//       }
//       LogicalExpression::Gt { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Gt, left.into(), right.into())
//       }
//       LogicalExpression::GtEq { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::GtEq, left.into(), right.into())
//       }
//       LogicalExpression::Add { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Add, left.into(), right.into())
//       }
//       LogicalExpression::Sub { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Sub, left.into(), right.into())
//       }
//       LogicalExpression::Mul { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Mul, left.into(), right.into())
//       }
//       LogicalExpression::Div { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::Div, left.into(), right.into())
//       }
//       LogicalExpression::StartsWith { left, right } => {
//         data::LogicalExpr::binary(data::BinaryOp::StartsWith, left.into(), right.into())
//       }
//     }
//   }
// }
