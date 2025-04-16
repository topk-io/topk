use super::{function_expr::FunctionExpr, logical_expr::LogicalExpr};
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

impl Into<topk_rs::data::select_expr::SelectExpr> for SelectExpr {
    fn into(self) -> topk_rs::data::select_expr::SelectExpr {
        match self {
            SelectExpr::Logical(expr) => {
                topk_rs::data::select_expr::SelectExpr::Logical(expr.into())
            }
            SelectExpr::Function(expr) => {
                topk_rs::data::select_expr::SelectExpr::Function(expr.into())
            }
        }
    }
}
