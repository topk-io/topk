use napi_derive::napi;

#[napi(string_enum = "camelCase", namespace = "query")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TernaryOperator {
    Choose,
}

impl Into<topk_rs::proto::v1::data::logical_expr::ternary_op::Op> for TernaryOperator {
    fn into(self) -> topk_rs::proto::v1::data::logical_expr::ternary_op::Op {
        match self {
            TernaryOperator::Choose => {
                topk_rs::proto::v1::data::logical_expr::ternary_op::Op::Choose
            }
        }
    }
}
