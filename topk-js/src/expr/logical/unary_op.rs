use napi_derive::napi;

#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    IsNull,
    IsNotNull,
    Abs,
}

impl Into<topk_rs::proto::v1::data::logical_expr::unary_op::Op> for UnaryOperator {
    fn into(self) -> topk_rs::proto::v1::data::logical_expr::unary_op::Op {
        match self {
            UnaryOperator::Not => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Not,
            UnaryOperator::IsNull => topk_rs::proto::v1::data::logical_expr::unary_op::Op::IsNull,
            UnaryOperator::IsNotNull => {
                topk_rs::proto::v1::data::logical_expr::unary_op::Op::IsNotNull
            }
            UnaryOperator::Abs => topk_rs::proto::v1::data::logical_expr::unary_op::Op::Abs,
        }
    }
}
