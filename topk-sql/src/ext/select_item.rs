use sqlparser::ast::{Expr as SqlExpr, SelectItem};

use super::SqlExprExt;
use crate::Error;

pub trait SelectItemExt {
    fn expr(&self) -> Option<&SqlExpr>;

    fn column_name(&self) -> String;

    fn projection_name(&self) -> Result<String, Error>;

    fn is_wildcard(&self) -> bool;
}

impl SelectItemExt for SelectItem {
    fn expr(&self) -> Option<&SqlExpr> {
        match self {
            SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => Some(expr),
            _ => None,
        }
    }

    fn column_name(&self) -> String {
        match self {
            SelectItem::ExprWithAlias { alias, .. } => alias.value.clone(),
            SelectItem::UnnamedExpr(expr) => expr.as_column_name(),
            _ => "?column?".to_string(),
        }
    }

    fn projection_name(&self) -> Result<String, Error> {
        match self {
            SelectItem::ExprWithAlias { alias, .. } => Ok(alias.value.clone()),
            SelectItem::UnnamedExpr(expr) => {
                if let Some(name) = expr.as_ident() {
                    return Ok(name);
                }

                match expr {
                    SqlExpr::Function(f) => f
                        .name
                        .0
                        .last()
                        .map(|i| i.value.clone())
                        .ok_or_else(|| Error::Invalid("function with no name".into())),
                    SqlExpr::Cast { expr, .. } => SelectItemExt::projection_name(expr),
                    _ => Err(Error::Invalid(
                        "expression in SELECT list requires an AS alias".into(),
                    )),
                }
            }
            _ => Err(Error::Unsupported("SELECT *".into())),
        }
    }

    fn is_wildcard(&self) -> bool {
        matches!(
            self,
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..)
        )
    }
}
