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

impl From<SelectExpr> for topk_rs::proto::v1::data::stage::select_stage::SelectExpr {
    fn from(expr: SelectExpr) -> Self {
        match expr {
            SelectExpr::Logical(expr) => {
                topk_rs::proto::v1::data::stage::select_stage::SelectExpr::logical(expr)
            }
            SelectExpr::Function(expr) => {
                topk_rs::proto::v1::data::stage::select_stage::SelectExpr::function(expr)
            }
        }
    }
}
