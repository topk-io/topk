use napi_derive::napi;

#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    StartsWith,
    Contains,
    In,
    Add,
    Sub,
    Mul,
    Div,
    MatchAll,
    MatchAny,
    Coalesce,
    Min,
    Max,
}

impl Into<topk_rs::proto::v1::data::logical_expr::binary_op::Op> for BinaryOperator {
    fn into(self) -> topk_rs::proto::v1::data::logical_expr::binary_op::Op {
        match self {
            BinaryOperator::And => topk_rs::proto::v1::data::logical_expr::binary_op::Op::And,
            BinaryOperator::Or => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Or,
            BinaryOperator::Eq => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Eq,
            BinaryOperator::Neq => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Neq,
            BinaryOperator::Lt => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Lt,
            BinaryOperator::Lte => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Lte,
            BinaryOperator::Gt => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Gt,
            BinaryOperator::Gte => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Gte,
            BinaryOperator::StartsWith => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::StartsWith
            }
            BinaryOperator::Contains => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::Contains
            }
            BinaryOperator::In => topk_rs::proto::v1::data::logical_expr::binary_op::Op::In,
            BinaryOperator::Add => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Add,
            BinaryOperator::Sub => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Sub,
            BinaryOperator::Mul => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Mul,
            BinaryOperator::Div => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Div,
            BinaryOperator::MatchAll => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::MatchAll
            }
            BinaryOperator::MatchAny => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::MatchAny
            }
            BinaryOperator::Coalesce => {
                topk_rs::proto::v1::data::logical_expr::binary_op::Op::Coalesce
            }
            BinaryOperator::Min => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Min,
            BinaryOperator::Max => topk_rs::proto::v1::data::logical_expr::binary_op::Op::Max,
        }
    }
}
