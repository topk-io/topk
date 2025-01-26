use super::{function_expr::FunctionExpression, logical_expr::LogicalExpression};
use pyo3::prelude::*;
use topk_protos::v1::data;

#[pyclass]
#[derive(Debug, Clone)]
pub enum SelectExpression {
    Logical(LogicalExpression),
    Function(FunctionExpression),
}

#[derive(Debug, Clone, FromPyObject)]
pub enum SelectExpressionUnion {
    #[pyo3(transparent)]
    Logical(LogicalExpression),

    #[pyo3(transparent)]
    Function(FunctionExpression),
}

impl Into<data::stage::select_stage::SelectExpr> for SelectExpression {
    fn into(self) -> data::stage::select_stage::SelectExpr {
        match self {
            SelectExpression::Logical(expr) => {
                data::stage::select_stage::SelectExpr::logical(expr.into())
            }
            SelectExpression::Function(expr) => {
                data::stage::select_stage::SelectExpr::function(expr.into())
            }
        }
    }
}
