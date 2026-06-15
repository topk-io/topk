use std::ops::ControlFlow;

use sqlparser::ast::{BinaryOperator, Expr as SqlExpr, visit_expressions};

use topk_rs::proto::v1::data::{LogicalExpr, TextExpr, stage::filter_stage::FilterExpr};

use super::Expr;
use crate::{Error, FromSql, SqlFunctionExt, sql_invalid, sql_unsupported};

impl FromSql<SqlExpr> for Vec<FilterExpr> {
    fn from_sql(expr: SqlExpr) -> Result<Vec<FilterExpr>, Error> {
        let mut text: Option<TextExpr> = None;
        let mut logical = Vec::new();

        // Split the expression on AND operators and separate text and logical expressions
        // This works naturally in other SDKs as `.filter()` accepts `LogicalExpr | TextExpr`.
        for conjunct in split_binary(expr, BinaryOperator::And) {
            if contains_text_expr(&conjunct) {
                text = Some(match text {
                    Some(left) => left.and(TextExpr::from_sql(conjunct)?),
                    None => TextExpr::from_sql(conjunct)?,
                });
            } else {
                logical.push(conjunct);
            }
        }

        // Collect logical expressions into an AND chain
        let logical = logical.into_iter().reduce(|left, right| SqlExpr::BinaryOp {
            left: Box::new(left),
            op: BinaryOperator::And,
            right: Box::new(right),
        });

        // Group text and logical expressions. Order doesn't matter as they are combined with AND.
        let filters = {
            let mut filters = Vec::with_capacity(2);
            if let Some(expr) = text {
                filters.push(FilterExpr::text(expr));
            }
            if let Some(expr) = logical {
                filters.push(FilterExpr::logical(LogicalExpr::from_sql(expr)?));
            }
            filters
        };

        Ok(filters)
    }
}

impl FromSql<SqlExpr> for TextExpr {
    fn from_sql(expr: SqlExpr) -> Result<TextExpr, Error> {
        match expr {
            SqlExpr::Nested(inner) => FromSql::from_sql(*inner),
            SqlExpr::Function(func)
                if func.name().eq_ignore_ascii_case("match")
                    || func.name().eq_ignore_ascii_case("match_tokens") =>
            {
                match Expr::try_from(func.clone())? {
                    Expr::Text(expr) => Ok(expr),
                    _ => sql_invalid!("Expected match or match_tokens function"),
                }
            }
            SqlExpr::Function(_) => sql_invalid!("Expected match or match_tokens function"),

            SqlExpr::BinaryOp { left, op, right }
                if matches!(op, BinaryOperator::And | BinaryOperator::Or) =>
            {
                sql_unsupported!(
                    contains_text_expr(&left) != contains_text_expr(&right),
                    "match/match_tokens can only be combined with logical filters using AND"
                );

                let left = TextExpr::from_sql(*left)?;
                let right = TextExpr::from_sql(*right)?;

                let expr = match op {
                    BinaryOperator::And => left.and(right),
                    BinaryOperator::Or => left.or(right),
                    _ => sql_invalid!("Expected AND or OR operator in text filter"),
                };

                Ok(expr)
            }

            _ => sql_invalid!("Expected match or match_tokens function"),
        }
    }
}

fn split_binary(expr: SqlExpr, op: BinaryOperator) -> Vec<SqlExpr> {
    match expr {
        SqlExpr::Nested(inner) => split_binary(*inner, op),
        SqlExpr::BinaryOp {
            left,
            op: expr_op,
            right,
        } if expr_op == op => {
            let mut exprs = split_binary(*left, op.clone());
            exprs.extend(split_binary(*right, op));
            exprs
        }
        expr => vec![expr.clone()],
    }
}

fn contains_text_expr(expr: &SqlExpr) -> bool {
    matches!(
        visit_expressions(expr, |expr| {
            if is_text_expr(expr) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }),
        ControlFlow::Break(())
    )
}

fn is_text_expr(expr: &SqlExpr) -> bool {
    match expr {
        SqlExpr::Nested(inner) => is_text_expr(inner),
        SqlExpr::Function(func) => {
            let name = func.name();
            name.eq_ignore_ascii_case("match") || name.eq_ignore_ascii_case("match_tokens")
        }
        _ => false,
    }
}
