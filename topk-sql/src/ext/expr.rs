use sqlparser::ast::{Expr as SqlExpr, Value as SqlValue};

use super::{SqlFunctionExt, SqlValueExt};
use crate::Error;

pub trait SqlExprExt {
    fn as_id(&self) -> Option<String>;
    fn as_ident(&self) -> Option<String>;
    fn as_column_name(&self) -> String;
    /// If this expression is a `$N` placeholder, return its 0-based index.
    fn as_placeholder(&self) -> Option<usize>;

    fn as_string(&self) -> Option<String>;
    fn string_literal(&self, what: &str) -> Result<String, Error>;
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

    fn as_placeholder(&self) -> Option<usize> {
        match self {
            SqlExpr::Value(v) => v.value.as_placeholder(),
            SqlExpr::Nested(inner) => inner.as_placeholder(),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            SqlExpr::Value(v) => match &v.value {
                SqlValue::SingleQuotedString(s) | SqlValue::DoubleQuotedString(s) => {
                    Some(s.clone())
                }
                _ => None,
            },
            SqlExpr::Nested(inner) => inner.as_string(),
            _ => None,
        }
    }

    fn string_literal(&self, what: &str) -> Result<String, Error> {
        self.as_string().ok_or_else(|| match self {
            SqlExpr::Identifier(ident) if ident.quote_style == Some('"') => {
                Error::QuotedString(ident.value.clone())
            }
            _ => Error::Invalid(format!("{what} must be a string literal")),
        })
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            SqlExpr::Value(v) => match v.value {
                SqlValue::Boolean(b) => Some(b),
                _ => None,
            },
            SqlExpr::Nested(inner) => inner.as_bool(),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            SqlExpr::Value(v) => match &v.value {
                SqlValue::Number(n, _) => n.parse::<u64>().ok(),
                _ => None,
            },
            SqlExpr::Nested(inner) => inner.as_u64(),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            SqlExpr::Value(v) => match &v.value {
                SqlValue::Number(n, _) => n.parse::<i64>().ok(),
                _ => None,
            },
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{convert_sql, parse_sql};

    #[rstest]
    #[case::match_query(
        "SELECT _id FROM t WHERE match(\"dune\", title) LIMIT 10",
        "Invalid: \"dune\" is an identifier; use single quotes for strings: 'dune'"
    )]
    #[case::regex_operator(
        "SELECT _id FROM t WHERE title ~ \"^D\" LIMIT 10",
        "Invalid: \"^D\" is an identifier; use single quotes for strings: '^D'"
    )]
    #[case::like(
        "SELECT _id FROM t WHERE title LIKE \"de-%\" LIMIT 10",
        "Invalid: \"de-%\" is an identifier; use single quotes for strings: 'de-%'"
    )]
    #[case::not_a_string(
        "SELECT _id FROM t WHERE regexp_like(title, 1) LIMIT 10",
        "Invalid: regexp_like: pattern must be a string literal"
    )]
    fn string_literal(#[case] sql: &str, #[case] message: &str) {
        let error = parse_sql(sql).and_then(convert_sql).unwrap_err();
        assert_eq!(error.to_string(), message);
    }
}
