use napi_derive::napi;

#[napi(string_enum = "lowercase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    // Logical ops
    And,
    Or,
    // Comparison ops
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    StartsWith,
    // Arithmetic ops
    Add,
    Sub,
    Mul,
    Div,
}

impl Into<topk_protos::v1::data::logical_expr::binary_op::Op> for BinaryOperator {
    fn into(self) -> topk_protos::v1::data::logical_expr::binary_op::Op {
        match self {
            BinaryOperator::And => topk_protos::v1::data::logical_expr::binary_op::Op::And,
            BinaryOperator::Or => topk_protos::v1::data::logical_expr::binary_op::Op::Or,
            BinaryOperator::Eq => topk_protos::v1::data::logical_expr::binary_op::Op::Eq,
            BinaryOperator::Neq => topk_protos::v1::data::logical_expr::binary_op::Op::Neq,
            BinaryOperator::Lt => topk_protos::v1::data::logical_expr::binary_op::Op::Lt,
            BinaryOperator::Lte => topk_protos::v1::data::logical_expr::binary_op::Op::Lte,
            BinaryOperator::Gt => topk_protos::v1::data::logical_expr::binary_op::Op::Gt,
            BinaryOperator::Gte => topk_protos::v1::data::logical_expr::binary_op::Op::Gte,
            BinaryOperator::StartsWith => {
                topk_protos::v1::data::logical_expr::binary_op::Op::StartsWith
            }
            BinaryOperator::Add => topk_protos::v1::data::logical_expr::binary_op::Op::Add,
            BinaryOperator::Sub => topk_protos::v1::data::logical_expr::binary_op::Op::Sub,
            BinaryOperator::Mul => topk_protos::v1::data::logical_expr::binary_op::Op::Mul,
            BinaryOperator::Div => topk_protos::v1::data::logical_expr::binary_op::Op::Div,
        }
    }
}

impl Into<topk_rs::expr::logical::BinaryOperator> for BinaryOperator {
    fn into(self) -> topk_rs::expr::logical::BinaryOperator {
        match self {
            BinaryOperator::And => topk_rs::expr::logical::BinaryOperator::And,
            BinaryOperator::Or => topk_rs::expr::logical::BinaryOperator::Or,
            BinaryOperator::Eq => topk_rs::expr::logical::BinaryOperator::Eq,
            BinaryOperator::Neq => topk_rs::expr::logical::BinaryOperator::NotEq,
            BinaryOperator::Lt => topk_rs::expr::logical::BinaryOperator::Lt,
            BinaryOperator::Lte => topk_rs::expr::logical::BinaryOperator::LtEq,
            BinaryOperator::Gt => topk_rs::expr::logical::BinaryOperator::Gt,
            BinaryOperator::Gte => topk_rs::expr::logical::BinaryOperator::GtEq,
            BinaryOperator::StartsWith => topk_rs::expr::logical::BinaryOperator::StartsWith,
            BinaryOperator::Add => topk_rs::expr::logical::BinaryOperator::Add,
            BinaryOperator::Sub => topk_rs::expr::logical::BinaryOperator::Sub,
            BinaryOperator::Mul => topk_rs::expr::logical::BinaryOperator::Mul,
            BinaryOperator::Div => topk_rs::expr::logical::BinaryOperator::Div,
        }
    }
}
