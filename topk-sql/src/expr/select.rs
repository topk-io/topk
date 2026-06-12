use sqlparser::ast::Expr as SqlExpr;
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::proto::v1::data::stage::select_stage::SelectExpr;

use super::SqlFn;
use crate::{Error, FromSql};

impl FromSql<SqlExpr> for SelectExpr {
    fn from_sql(expr: SqlExpr) -> Result<SelectExpr, Error> {
        match expr {
            SqlExpr::Function(func) => match SqlFn::try_from(func)? {
                SqlFn::Function(func) => Ok(SelectExpr::function(func)),
                SqlFn::Logical(expr) | SqlFn::TextMatch { logical: expr, .. } => {
                    Ok(SelectExpr::logical(expr))
                }
                SqlFn::Literal(value) => Ok(SelectExpr::logical(LogicalExpr::literal(value))),
            },
            SqlExpr::Cast { expr, .. } => Ok(SelectExpr::from_sql(*expr)?),
            other => Ok(SelectExpr::logical(LogicalExpr::from_sql(other)?)),
        }
    }
}
