use napi_derive::napi;

#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    IsNull,
    IsNotNull,
}

impl Into<topk_rs::proto::v1::data::logical_expr::unary_op::Op> for UnaryOperator {
    fn into(self) -> topk_rs::proto::v1::data::logical_expr::unary_op::Op {
        match self {
            UnaryOperator::Not => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Not,
            UnaryOperator::IsNull => topk_rs::proto::v1::data::logical_expr::unary_op::Op::IsNull,
            UnaryOperator::IsNotNull => {
                topk_rs::proto::v1::data::logical_expr::unary_op::Op::IsNotNull
            }
        }
    }
}

impl Into<topk_rs::expr::logical::UnaryOperator> for UnaryOperator {
    fn into(self) -> topk_rs::expr::logical::UnaryOperator {
        match self {
            UnaryOperator::Not => topk_rs::expr::logical::UnaryOperator::Not,
            UnaryOperator::IsNull => topk_rs::expr::logical::UnaryOperator::IsNull,
            UnaryOperator::IsNotNull => topk_rs::expr::logical::UnaryOperator::IsNotNull,
        }
    }
}
