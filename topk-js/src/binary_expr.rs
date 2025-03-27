use napi_derive::napi;

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
  // Logical ops
  And,
  Or,
  Xor,
  // Comparison ops
  Eq,
  NotEq,
  Lt,
  LtEq,
  Gt,
  GtEq,
  StartsWith,
  // Arithmetic ops
  Add,
  Sub,
  Mul,
  Div,
  Rem,
}

impl Into<topk_protos::v1::data::logical_expr::binary_op::Op> for BinaryOperator {
  fn into(self) -> topk_protos::v1::data::logical_expr::binary_op::Op {
    match self {
      BinaryOperator::Eq => topk_protos::v1::data::logical_expr::binary_op::Op::Eq,
      BinaryOperator::NotEq => topk_protos::v1::data::logical_expr::binary_op::Op::Neq,
      BinaryOperator::Lt => topk_protos::v1::data::logical_expr::binary_op::Op::Lt,
      BinaryOperator::LtEq => topk_protos::v1::data::logical_expr::binary_op::Op::Lte,
      BinaryOperator::Gt => topk_protos::v1::data::logical_expr::binary_op::Op::Gt,
      BinaryOperator::GtEq => topk_protos::v1::data::logical_expr::binary_op::Op::Gte,
      BinaryOperator::StartsWith => topk_protos::v1::data::logical_expr::binary_op::Op::StartsWith,
      BinaryOperator::Add => topk_protos::v1::data::logical_expr::binary_op::Op::Add,
      BinaryOperator::Sub => topk_protos::v1::data::logical_expr::binary_op::Op::Sub,
      BinaryOperator::Mul => topk_protos::v1::data::logical_expr::binary_op::Op::Mul,
      BinaryOperator::Div => topk_protos::v1::data::logical_expr::binary_op::Op::Div,
      BinaryOperator::And => topk_protos::v1::data::logical_expr::binary_op::Op::And,
      BinaryOperator::Or => topk_protos::v1::data::logical_expr::binary_op::Op::Or,
      BinaryOperator::Xor => unreachable!("Xor is not supported"),
      BinaryOperator::Rem => unreachable!("Rem is not supported"),
    }
  }
}
