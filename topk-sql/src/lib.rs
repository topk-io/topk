use std::ops::ControlFlow;
use std::sync::LazyLock;

use regex::Regex;
use sqlparser::ast::{self, visit_expressions};
use sqlparser::parser::ParserError;

mod dialect;
use dialect::TopKDialect;

mod ext;
pub use ext::{
    ObjectNameExt, SelectItemExt, SqlExprExt, SqlFunctionExt, SqlStatementExt, TableFactorExt,
};

mod expr;
pub use expr::Expr;

mod stmt;
pub use stmt::{RowFilter, Statement, Variable};

mod table;
pub use table::Table;

pub mod util;

pub trait FromSql<S>: Sized {
    fn from_sql(value: S) -> Result<Self, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Parse error: {0}")]
    Parse(#[from] ParserError),

    #[error("TopK error: {0}")]
    Topk(#[from] topk_rs::Error),

    #[error("Unsupported: {0}")]
    Unsupported(String),

    #[error("Invalid: {0}")]
    Invalid(String),

    #[error("Invalid: {0}")]
    InvalidLiteral(String),

    #[error("Unknown function: {0}")]
    UnknownFunction(String),

    #[error("Invalid JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Internal: {0}")]
    Internal(String),
}

/// Parse a SQL string into a list of statements.
pub fn parse_sql(sql: &str) -> Result<Vec<ast::Statement>, Error> {
    // Rewrite non-standard syntax
    let sql = rewrite_partition_syntax(sql);

    // Parse
    let dialect = TopKDialect::default();
    let stmts = sqlparser::parser::Parser::parse_sql(&dialect, &sql)?;

    // Validate
    let mut diag = Vec::new();
    for stmt in stmts.iter() {
        let _: ControlFlow<()> = visit_expressions(stmt, |expr| {
            // Validate unsupported expressions.
            match expr {
                ast::Expr::ILike { .. } => {
                    diag.push("ILIKE: TopK has no case-insensitive matching primitive");
                }
                ast::Expr::IsDistinctFrom(_, _) => {
                    diag.push("IS DISTINCT FROM: not supported");
                }
                ast::Expr::IsNotDistinctFrom(_, _) => {
                    diag.push("IS NOT DISTINCT FROM: not supported");
                }
                ast::Expr::Subquery(_) | ast::Expr::Exists { .. } => {
                    diag.push("Subqueries are not supported");
                }
                ast::Expr::InSubquery { .. } => {
                    diag.push("IN (SELECT …): not supported");
                }
                ast::Expr::InUnnest { .. } => {
                    diag.push("IN UNNEST(…): not supported");
                }
                _ => {}
            }

            ControlFlow::Continue(())
        });
    }

    if !diag.is_empty() {
        return Err(Error::Unsupported(diag.join("; ")));
    }

    Ok(stmts)
}

/// Convert a parsed SQL batch into typed statements.
///
/// Returns pairs of `(Statement, Option<ast::Statement>)` where the raw SQL is
/// preserved only for `Query` statements (needed by callers for projection inference).
pub fn convert_sql(
    batch: Vec<ast::Statement>,
) -> Result<Vec<(Statement, Option<ast::Statement>)>, Error> {
    batch
        .into_iter()
        .map(|sql| {
            let raw = matches!(sql, ast::Statement::Query(_)).then(|| sql.clone());
            Statement::try_from(sql).map(|stmt| (stmt, raw))
        })
        .collect()
}

// Rewrite "SELECT * FROM books PARTITION p1" → "SELECT * FROM books$p1"
//
// NON-STANDARD SYNTAX: TopK extends PostgreSQL syntax with a PARTITION clause on table
// references. This is not valid PostgreSQL — we rewrite it before handing off to sqlparser
// so the rest of the pipeline sees only the canonical `collection$partition` form (a `$`-
// qualified identifier, which PostgreSQL dialects accept).
fn rewrite_partition_syntax(sql: &str) -> std::borrow::Cow<'_, str> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b([\w.]+)\s+PARTITION\s+(\w+)\b").unwrap());

    RE.replace_all(sql, |caps: &regex::Captures| {
        let partition = &caps[2];
        if partition.eq_ignore_ascii_case("BY") {
            caps[0].to_string()
        } else {
            format!("{}${}", &caps[1], partition)
        }
    })
}

#[macro_export]
macro_rules! sql_unsupported {
    ($cond:expr, $($arg:tt)+) => {
        if $cond {
            return Err($crate::Error::Unsupported(format!($($arg)+)).into());
        }
    };
    ($($arg:tt)+) => {
        return Err($crate::Error::Unsupported(format!($($arg)+)).into())
    };
}

#[macro_export]
macro_rules! sql_invalid {
    ($cond:expr, $($arg:tt)+) => {
        if $cond {
            return Err($crate::Error::Invalid(format!($($arg)+)).into());
        }
    };
    ($($arg:tt)+) => {
        return Err($crate::Error::Invalid(format!($($arg)+)).into())
    };
}
