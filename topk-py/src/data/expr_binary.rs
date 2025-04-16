use pyo3::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(eq, eq_int)]
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

impl Into<topk_rs::data::expr_binary::BinaryOperator> for BinaryOperator {
    fn into(self) -> topk_rs::data::expr_binary::BinaryOperator {
        match self {
            BinaryOperator::Eq => topk_rs::data::expr_binary::BinaryOperator::Eq,
            BinaryOperator::NotEq => topk_rs::data::expr_binary::BinaryOperator::NotEq,
            BinaryOperator::Lt => topk_rs::data::expr_binary::BinaryOperator::Lt,
            BinaryOperator::LtEq => topk_rs::data::expr_binary::BinaryOperator::LtEq,
            BinaryOperator::Gt => topk_rs::data::expr_binary::BinaryOperator::Gt,
            BinaryOperator::GtEq => topk_rs::data::expr_binary::BinaryOperator::GtEq,
            BinaryOperator::StartsWith => topk_rs::data::expr_binary::BinaryOperator::StartsWith,
            BinaryOperator::Add => topk_rs::data::expr_binary::BinaryOperator::Add,
            BinaryOperator::Sub => topk_rs::data::expr_binary::BinaryOperator::Sub,
            BinaryOperator::Mul => topk_rs::data::expr_binary::BinaryOperator::Mul,
            BinaryOperator::Div => topk_rs::data::expr_binary::BinaryOperator::Div,
            BinaryOperator::And => topk_rs::data::expr_binary::BinaryOperator::And,
            BinaryOperator::Or => topk_rs::data::expr_binary::BinaryOperator::Or,
            BinaryOperator::Xor => unreachable!("Xor is not supported"),
            BinaryOperator::Rem => unreachable!("Rem is not supported"),
        }
    }
}
