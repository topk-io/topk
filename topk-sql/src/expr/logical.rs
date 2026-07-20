use sqlparser::ast::{BinaryOperator, Expr as SqlExpr, UnaryOperator};
use topk_rs::proto::v1::data::{LogicalExpr, Value};

use crate::expr::regexp;
use crate::{Error, FromSql, SqlExprExt, sql_invalid, sql_unsupported};

impl FromSql<SqlExpr> for LogicalExpr {
    fn from_sql(expr: SqlExpr) -> Result<LogicalExpr, Error> {
        match expr {
            SqlExpr::Identifier(ident) => Ok(LogicalExpr::field(ident.value)),
            SqlExpr::CompoundIdentifier(parts) => Ok(LogicalExpr::field(
                parts
                    .into_iter()
                    .map(|p| p.value)
                    .collect::<Vec<_>>()
                    .join("."),
            )),
            SqlExpr::Value(_) => Ok(LogicalExpr::literal(Value::from_sql(expr)?)),
            SqlExpr::Array(arr) => Ok(LogicalExpr::literal(Value::from_sql(arr.elem)?)),
            SqlExpr::Nested(inner) => LogicalExpr::from_sql(*inner),
            SqlExpr::UnaryOp { op, expr } => match op {
                UnaryOperator::Not => Ok(LogicalExpr::not(LogicalExpr::from_sql(*expr)?)),
                UnaryOperator::Minus => {
                    Ok(LogicalExpr::literal(Value::i64(0)).sub(LogicalExpr::from_sql(*expr)?))
                }
                UnaryOperator::Plus => LogicalExpr::from_sql(*expr),
                other => sql_unsupported!("unary operator {other:?}"),
            },

            SqlExpr::BinaryOp { left, op, right } => {
                // (pg_flags, negated)
                let regexp_op = match op {
                    BinaryOperator::PGRegexMatch => Some(("", false)),
                    BinaryOperator::PGRegexIMatch => Some(("i", false)),
                    BinaryOperator::PGRegexNotMatch => Some(("", true)),
                    BinaryOperator::PGRegexNotIMatch => Some(("i", true)),
                    _ => None,
                };
                if let Some((pg_flags, negated)) = regexp_op {
                    let pat = right.as_string().ok_or(Error::Unsupported(
                        "regex pattern must be a string literal".to_string(),
                    ))?;
                    let (pattern, flags) = regexp::translate(&pat, pg_flags)?;
                    let expr = LogicalExpr::from_sql(*left)?;
                    let matches = expr.clone().regexp_match(pattern, flags);
                    // PG's negated operators return NULL for NULL input, which
                    // filters the row out; a bare NOT would match it instead.
                    return Ok(if negated {
                        expr.is_not_null().and(LogicalExpr::not(matches))
                    } else {
                        matches
                    });
                }

                let l = LogicalExpr::from_sql(*left)?;
                let r = LogicalExpr::from_sql(*right)?;

                Ok(match op {
                    BinaryOperator::Plus => l.add(r),
                    BinaryOperator::Minus => l.sub(r),
                    BinaryOperator::Multiply => l.mul(r),
                    BinaryOperator::Divide => l.div(r),
                    BinaryOperator::Eq => l.eq(r),
                    BinaryOperator::NotEq => l.neq(r),
                    BinaryOperator::Lt => l.lt(r),
                    BinaryOperator::LtEq => l.lte(r),
                    BinaryOperator::Gt => l.gt(r),
                    BinaryOperator::GtEq => l.gte(r),
                    BinaryOperator::And => l.and(r),
                    BinaryOperator::Or => l.or(r),
                    other => sql_unsupported!("binary operator {other:?}"),
                })
            }

            SqlExpr::IsNull(inner) => Ok(LogicalExpr::from_sql(*inner)?.is_null()),
            SqlExpr::IsNotNull(inner) => Ok(LogicalExpr::from_sql(*inner)?.is_not_null()),

            SqlExpr::InList {
                expr,
                list,
                negated,
            } => {
                sql_invalid!(list.is_empty(), "IN () with empty list");

                let field = LogicalExpr::from_sql(*expr)?;
                let list = Value::from_sql(list).map_err(|e| match e {
                    Error::Unsupported(msg) if msg.starts_with("expression as value:") => {
                        Error::Unsupported("IN list must contain only literal values".to_string())
                    }
                    other => other,
                })?;

                let expr = field.in_(list);
                Ok(if negated {
                    LogicalExpr::not(expr)
                } else {
                    expr
                })
            }

            SqlExpr::Between {
                expr,
                low,
                high,
                negated,
            } => {
                let value = LogicalExpr::from_sql(*expr)?;
                let lo = LogicalExpr::from_sql(*low)?;
                let hi = LogicalExpr::from_sql(*high)?;
                let bounded = value.clone().gte(lo).and(value.lte(hi));
                Ok(if negated {
                    LogicalExpr::not(bounded)
                } else {
                    bounded
                })
            }

            SqlExpr::Like {
                negated,
                expr,
                pattern,
                escape_char,
                ..
            } => {
                sql_unsupported!(escape_char.is_some(), "LIKE ESCAPE clause");

                // Parse LIKE pattern.
                let pattern: String = Value::from_sql(*pattern)?
                    .as_string()
                    .map(String::from)
                    .ok_or(Error::Unsupported(
                        "LIKE pattern must be a string".to_string(),
                    ))?;

                // Parse target expression.
                let expr = LogicalExpr::from_sql(*expr)?;
                let expr =
                    if pattern.starts_with('%') && pattern.ends_with('%') && pattern.len() >= 2 {
                        let inner = &pattern[1..pattern.len() - 1];
                        sql_unsupported!(
                            inner.contains('%') || inner.contains('_'),
                            "LIKE pattern `{pattern}` contains an unsupported wildcard"
                        );
                        expr.contains(Value::string(inner.to_string()))
                    } else if let Some(prefix) = pattern.strip_suffix('%') {
                        sql_unsupported!(
                            prefix.contains('%') || prefix.contains('_'),
                            "LIKE pattern `{pattern}` contains an unsupported wildcard"
                        );
                        expr.starts_with(Value::string(prefix.to_string()))
                    } else if !pattern.contains('%') && !pattern.contains('_') {
                        expr.eq(Value::string(pattern))
                    } else {
                        sql_unsupported!("LIKE pattern `{pattern}` is not supported");
                    };

                Ok(if negated {
                    LogicalExpr::not(expr)
                } else {
                    expr
                })
            }

            // `CASE WHEN c THEN r [ELSE e] END` → right-folded OP_CHOOSE.
            // `CASE x WHEN v THEN r END` → `CASE WHEN x = v THEN r END`.
            SqlExpr::Case {
                operand,
                conditions,
                else_result,
                ..
            } => {
                sql_invalid!(
                    conditions.is_empty(),
                    "CASE requires at least one WHEN/THEN pair"
                );

                let pairs: Vec<(SqlExpr, SqlExpr)> = match operand {
                    Some(op) => conditions
                        .into_iter()
                        .map(|cw| {
                            (
                                SqlExpr::BinaryOp {
                                    left: op.clone(),
                                    op: BinaryOperator::Eq,
                                    right: Box::new(cw.condition),
                                },
                                cw.result,
                            )
                        })
                        .collect(),
                    None => conditions
                        .into_iter()
                        .map(|cw| (cw.condition, cw.result))
                        .collect(),
                };

                let else_expr = match else_result {
                    Some(e) => LogicalExpr::from_sql(*e)?,
                    None => LogicalExpr::literal(Value::null()),
                };
                let mut acc = else_expr;
                for (cond, then) in pairs.into_iter().rev() {
                    let cond_e = LogicalExpr::from_sql(cond)?;
                    let then_e = LogicalExpr::from_sql(then)?;
                    acc = cond_e.choose(then_e, acc);
                }
                Ok(acc)
            }

            SqlExpr::Function(func) => LogicalExpr::from_sql(func),
            SqlExpr::Cast { .. } => sql_unsupported!("explicit CAST is only supported in SELECT"),

            other => sql_unsupported!("expression: {other:?}"),
        }
    }
}
