use sqlparser::ast::{Expr as SqlExpr, Value as SqlValue};

use super::SqlFunctionExt;

pub trait SqlExprExt {
    fn as_id(&self) -> Option<String>;
    fn as_ident(&self) -> Option<String>;
    fn as_column_name(&self) -> String;

    fn as_string(&self) -> Option<String>;
    fn as_bool(&self) -> Option<bool>;
    fn as_u64(&self) -> Option<u64>;
    fn as_i64(&self) -> Option<i64>;
}

impl SqlExprExt for SqlExpr {
    fn as_id(&self) -> Option<String> {
        match self {
            SqlExpr::Identifier(i) if i.value == "_id" => Some(i.value.clone()),
            _ => None,
        }
    }

    fn as_ident(&self) -> Option<String> {
        match self {
            SqlExpr::Identifier(i) => Some(i.value.clone()),
            SqlExpr::CompoundIdentifier(parts) => Some(
                parts
                    .iter()
                    .map(|p| p.value.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            _ => None,
        }
    }

    fn as_column_name(&self) -> String {
        // Parse as an `Ident` (eg. column name).
        if let Some(name) = self.as_ident() {
            return name;
        }

        match self {
            // COUNT(*) output column is named "_count" by convention.
            SqlExpr::Function(f) if f.is_count() => "_count".to_string(),
            SqlExpr::Function(f) => f.name.to_string(),
            SqlExpr::Cast { expr, .. } => expr.as_column_name(),
            _ => "?column?".to_string(),
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            SqlExpr::Value(SqlValue::SingleQuotedString(s) | SqlValue::DoubleQuotedString(s)) => {
                Some(s.clone())
            }
            SqlExpr::Nested(inner) => inner.as_string(),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            SqlExpr::Value(SqlValue::Boolean(b)) => Some(*b),
            SqlExpr::Nested(inner) => inner.as_bool(),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            SqlExpr::Value(SqlValue::Number(n, _)) => n.parse::<u64>().ok(),
            SqlExpr::Nested(inner) => inner.as_u64(),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            SqlExpr::Value(SqlValue::Number(n, _)) => n.parse::<i64>().ok(),
            SqlExpr::Nested(inner) => inner.as_i64(),
            // Negative integer literals are parsed as `UnaryOp { Minus, Number }`.
            SqlExpr::UnaryOp {
                op: sqlparser::ast::UnaryOperator::Minus,
                expr,
            } => expr.as_i64().and_then(|n| n.checked_neg()),
            _ => None,
        }
    }
}
