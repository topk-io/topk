use crate::expr::{function::FunctionExpr, logical::LogicalExpr};
use topk_protos::v1::data;

#[derive(Debug, Clone)]
pub enum SelectExpr {
    Logical(LogicalExpr),
    Function(FunctionExpr),
}

impl From<LogicalExpr> for SelectExpr {
    fn from(expr: LogicalExpr) -> Self {
        SelectExpr::Logical(expr)
    }
}

impl From<FunctionExpr> for SelectExpr {
    fn from(expr: FunctionExpr) -> Self {
        SelectExpr::Function(expr)
    }
}

impl Into<data::stage::select_stage::SelectExpr> for SelectExpr {
    fn into(self) -> data::stage::select_stage::SelectExpr {
        match self {
            SelectExpr::Logical(expr) => {
                data::stage::select_stage::SelectExpr::logical(expr.into())
            }
            SelectExpr::Function(expr) => {
                data::stage::select_stage::SelectExpr::function(expr.into())
            }
        }
    }
}
