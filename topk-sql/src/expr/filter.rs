use sqlparser::ast::{BinaryOperator, Expr as SqlExpr};

use topk_rs::proto::v1::data::{LogicalExpr, TextExpr, stage::filter_stage::FilterExpr};

use crate::{Error, FromSql, SqlFn, SqlFunctionExt, sql_invalid};

impl FromSql<SqlExpr> for FilterExpr {
    fn from_sql(expr: SqlExpr) -> Result<FilterExpr, Error> {
        match try_text_filter(&expr)? {
            Some(expr) => Ok(FilterExpr::text(expr)),
            None => Ok(FilterExpr::logical(LogicalExpr::from_sql(expr)?)),
        }
    }
}

fn try_text_filter(expr: &SqlExpr) -> Result<Option<TextExpr>, Error> {
    match expr {
        SqlExpr::Nested(inner) => try_text_filter(inner),
        SqlExpr::Function(func) => match func.name().as_str() {
            "match_all" | "match_any" => match SqlFn::try_from(func.clone()) {
                Ok(SqlFn::TextMatch { text, .. }) => Ok(text),
                _ => sql_invalid!("Expected match_all or match_any function"),
            },
            _ => Ok(None),
        },

        SqlExpr::BinaryOp { left, op, right }
            if matches!(op, BinaryOperator::And | BinaryOperator::Or) =>
        {
            let left = match try_text_filter(left)? {
                Some(left) => left,
                None => return Ok(None),
            };
            let right = match try_text_filter(right)? {
                Some(right) => right,
                None => return Ok(None),
            };

            // Build expr
            let expr = match op {
                BinaryOperator::And => left.and(right),
                BinaryOperator::Or => left.or(right),
                _ => return Ok(None),
            };

            Ok(Some(expr))
        }

        _ => Ok(None),
    }
}
