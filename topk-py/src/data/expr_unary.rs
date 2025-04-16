use pyo3::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
#[pyclass(eq, eq_int)]
pub enum UnaryOperator {
    Not,
    IsNull,
    IsNotNull,
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
