use crate::proto::{
    data::v1::Value,
    v1::data::{stage, FunctionExpr, LogicalExpr},
};

impl stage::select_stage::SelectExpr {
    pub fn logical(expr: impl Into<LogicalExpr>) -> Self {
        stage::select_stage::SelectExpr {
            expr: Some(stage::select_stage::select_expr::Expr::LogicalExpr(
                expr.into(),
            )),
        }
    }

    pub fn function(expr: impl Into<FunctionExpr>) -> Self {
        stage::select_stage::SelectExpr {
            expr: Some(stage::select_stage::select_expr::Expr::FunctionExpr(
                expr.into(),
            )),
        }
    }
}

impl From<LogicalExpr> for stage::select_stage::SelectExpr {
    fn from(expr: LogicalExpr) -> Self {
        stage::select_stage::SelectExpr {
            expr: Some(stage::select_stage::select_expr::Expr::LogicalExpr(expr)),
        }
    }
}

impl From<FunctionExpr> for stage::select_stage::SelectExpr {
    fn from(expr: FunctionExpr) -> Self {
        stage::select_stage::SelectExpr {
            expr: Some(stage::select_stage::select_expr::Expr::FunctionExpr(expr)),
        }
    }
}

impl From<Value> for stage::select_stage::SelectExpr {
    fn from(expr: Value) -> Self {
        stage::select_stage::SelectExpr::logical(expr)
    }
}
