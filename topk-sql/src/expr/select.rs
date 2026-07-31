use sqlparser::ast::Expr as SqlExpr;
use topk_rs::proto::v1::data::LogicalExpr;
use topk_rs::proto::v1::data::stage::select_stage::SelectExpr;

use crate::{Error, FromSql};

impl FromSql<SqlExpr> for SelectExpr {
    fn from_sql(expr: SqlExpr) -> Result<SelectExpr, Error> {
        match expr {
            SqlExpr::Cast { expr, .. } => SelectExpr::from_sql(*expr),
            other => Ok(SelectExpr::logical(LogicalExpr::from_sql(other)?)),
        }
    }
}
