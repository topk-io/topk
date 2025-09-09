#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NaryOp {
    All,
    Any,
}

impl Into<topk_rs::proto::v1::data::logical_expr::nary_op::Op> for NaryOp {
    fn into(self) -> topk_rs::proto::v1::data::logical_expr::nary_op::Op {
        match self {
            NaryOp::All => topk_rs::proto::v1::data::logical_expr::nary_op::Op::All,
            NaryOp::Any => topk_rs::proto::v1::data::logical_expr::nary_op::Op::Any,
        }
    }
}
