use crate::proto::v1::data::{stage, LogicalExpr, TextExpr};

impl stage::filter_stage::FilterExpr {
    pub fn logical(expr: impl Into<LogicalExpr>) -> Self {
        stage::filter_stage::FilterExpr {
            expr: Some(stage::filter_stage::filter_expr::Expr::LogicalExpr(
                expr.into(),
            )),
        }
    }

    pub fn text(expr: impl Into<TextExpr>) -> Self {
        stage::filter_stage::FilterExpr {
            expr: Some(stage::filter_stage::filter_expr::Expr::TextExpr(
                expr.into(),
            )),
        }
    }
}

impl From<LogicalExpr> for stage::filter_stage::FilterExpr {
    fn from(expr: LogicalExpr) -> Self {
        stage::filter_stage::FilterExpr::logical(expr)
    }
}

impl From<TextExpr> for stage::filter_stage::FilterExpr {
    fn from(expr: TextExpr) -> Self {
        stage::filter_stage::FilterExpr::text(expr)
    }
}
