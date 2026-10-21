use sqlparser::ast::Expr as SqlExpr;
use topk_rs::proto::v1::data::stage::select_stage::SelectExpr;
use topk_rs::proto::v1::data::LogicalExpr;

use super::Expr;
use crate::{sql_unsupported, Error, FromSql};

impl FromSql<SqlExpr> for SelectExpr {
    fn from_sql(expr: SqlExpr) -> Result<SelectExpr, Error> {
        match expr {
            SqlExpr::Function(func) => match Expr::try_from(func)? {
                Expr::Function(func) => Ok(SelectExpr::function(func)),
                Expr::Logical(expr) => Ok(SelectExpr::logical(expr)),
                Expr::Literal(value) => Ok(SelectExpr::logical(LogicalExpr::literal(value))),
                Expr::Text(_) => {
                    sql_unsupported!("`match` is a text filter function — only valid in WHERE")
                }
            },
            SqlExpr::Cast { expr, .. } => Ok(SelectExpr::from_sql(*expr)?),
            other => Ok(SelectExpr::logical(LogicalExpr::from_sql(other)?)),
        }
    }
}
