use sqlparser::ast::{BinaryOperator, Expr, Select, TableFactor};

use crate::{Error, FromSql, SelectItemExt, SqlExprExt, sql_invalid, stmt::Statement};

pub fn try_from_select(select: &Select) -> Option<Result<Statement, Error>> {
    let twj = select.from.first()?;

    if !twj.joins.is_empty() {
        return None;
    }

    let table = match &twj.relation {
        TableFactor::Table { name, .. } => {
            let table = name
                .0
                .last()
                .map(|i| i.value.to_ascii_lowercase())
                .unwrap_or_default();

            let schema = name
                .0
                .iter()
                .rev()
                .nth(1)
                .map(|i| i.value.to_ascii_lowercase());

            // Only information_schema is supported
            match schema.as_deref() {
                Some("information_schema") => {}
                _ => return None,
            }

            table
        }
        _ => return None,
    };

    let has_wildcard = select.projection.iter().any(|item| item.is_wildcard());
    if has_wildcard {
        return Some(Err(Error::Invalid(
            "SELECT * is not supported for information_schema; specify column names explicitly"
                .into(),
        )));
    }

    Some(match table.as_str() {
        "tables" => Ok(Statement::InfoSchemaTables),
        "columns" => {
            let result = select
                .selection
                .as_ref()
                .and_then(|e| TableName::from_sql(e.clone()).ok())
                .map(|t| t.0)
                .ok_or_else(|| {
                    crate::Error::Invalid(
                        "information_schema.columns requires WHERE table_name = '<name>'".into(),
                    )
                });
            result.map(|table| Statement::InfoSchemaColumns { table })
        }
        other => Err(Error::Unsupported(format!(
            "unknown information_schema table: {other}"
        ))),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableName(pub String);

impl FromSql<Expr> for TableName {
    fn from_sql(expr: Expr) -> Result<Self, Error> {
        match expr {
            Expr::BinaryOp {
                left,
                op: BinaryOperator::Eq,
                right,
            } => {
                let is_table = left
                    .as_ident()
                    .map(|s| s.eq_ignore_ascii_case("table_name"))
                    .unwrap_or(false);
                sql_invalid!(!is_table, "expected `WHERE table_name = <name>`");

                match right.as_ref() {
                    Expr::Value(sqlparser::ast::Value::SingleQuotedString(s)) => {
                        Ok(TableName(s.clone()))
                    }
                    _ => sql_invalid!("table_name must be a string literal"),
                }
            }
            Expr::BinaryOp {
                left,
                op: BinaryOperator::And,
                right,
            } => TableName::from_sql(*left).or_else(|_| TableName::from_sql(*right)),
            Expr::Nested(inner) => TableName::from_sql(*inner),
            _ => sql_invalid!("expected `WHERE table_name = <name>`"),
        }
    }
}
