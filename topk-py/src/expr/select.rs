use super::function::FunctionExpr;
use super::logical::LogicalExpr;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub enum SelectExpr {
    Logical(LogicalExpr),
    Function(FunctionExpr),
}

#[derive(Debug, Clone, FromPyObject)]
pub enum SelectExprUnion {
    #[pyo3(transparent)]
    Logical(LogicalExpr),

    #[pyo3(transparent)]
    Function(FunctionExpr),
}

impl Into<topk_rs::expr::select::SelectExpr> for SelectExpr {
    fn into(self) -> topk_rs::expr::select::SelectExpr {
        match self {
            SelectExpr::Logical(expr) => topk_rs::expr::select::SelectExpr::Logical(expr.into()),
            SelectExpr::Function(expr) => topk_rs::expr::select::SelectExpr::Function(expr.into()),
        }
    }
}
