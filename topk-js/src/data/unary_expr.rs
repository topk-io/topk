use napi_derive::napi;

#[napi(string_enum = "lowercase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    IsNull,
    IsNotNull,
}

impl Into<topk_protos::v1::data::logical_expr::unary_op::Op> for UnaryOperator {
    fn into(self) -> topk_protos::v1::data::logical_expr::unary_op::Op {
        match self {
            UnaryOperator::Not => topk_protos::v1::data::logical_expr::unary_op::Op::Not,
            UnaryOperator::IsNull => topk_protos::v1::data::logical_expr::unary_op::Op::IsNull,
            UnaryOperator::IsNotNull => {
                topk_protos::v1::data::logical_expr::unary_op::Op::IsNotNull
            }
        }
    }
}

impl Into<topk_rs::data::expr_unary::UnaryOperator> for UnaryOperator {
    fn into(self) -> topk_rs::data::expr_unary::UnaryOperator {
        match self {
            UnaryOperator::Not => topk_rs::data::expr_unary::UnaryOperator::Not,
            UnaryOperator::IsNull => topk_rs::data::expr_unary::UnaryOperator::IsNull,
            UnaryOperator::IsNotNull => topk_rs::data::expr_unary::UnaryOperator::IsNotNull,
        }
    }
}
