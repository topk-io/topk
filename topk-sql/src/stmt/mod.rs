use std::collections::HashMap;

use sqlparser::ast::{BinaryOperator, Expr as SqlExpr, Statement as SqlStatement};
use strum_macros::IntoStaticStr;
use topk_rs::proto::v1::control::FieldSpec;
use topk_rs::proto::v1::data::{Document, LogicalExpr, Query, Value};

use crate::{Error, FromSql, Index, SqlExprExt, Table, sql_invalid, sql_unsupported};

mod create_table;
mod delete;
mod drop;
mod explain;
mod info;
mod insert;
mod select;
mod set_variable;
mod show;
mod update;
mod variable;
pub use variable::Variable;

/// Convert a parsed SQL batch, folding consecutive `CREATE INDEX` into each
/// preceding `CREATE TABLE` schema. Stray `CREATE INDEX` statements error.
///
/// Returns pairs of `(Statement, Option<SqlStatement>)` where the raw SQL is
/// preserved only for `SELECT` queries (needed by callers for projection inference).
pub fn aggregate_stmts(
    batch: Vec<SqlStatement>,
) -> Result<Vec<(Statement, Option<SqlStatement>)>, Error> {
    let mut out = Vec::with_capacity(batch.len());

    let mut iter = batch.into_iter().peekable();
    while let Some(sql) = iter.next() {
        let (stmt, raw) = match sql {
            SqlStatement::CreateTable(ct) => {
                let Statement::CreateTable {
                    table,
                    mut schema,
                    if_not_exists,
                } = Statement::try_from(ct)?
                else {
                    unreachable!()
                };

                while matches!(iter.peek(), Some(SqlStatement::CreateIndex(_))) {
                    let index = match iter.next() {
                        Some(SqlStatement::CreateIndex(idx)) => Index::from_sql(idx)?,
                        _ => unreachable!(),
                    };

                    sql_invalid!(
                        index.table != table,
                        "CREATE INDEX references unknown table `{}`",
                        index.table
                    );
                    let field = &index.field;
                    let spec = match schema.get_mut(field) {
                        Some(spec) => spec,
                        None => sql_invalid!("CREATE INDEX references unknown field `{field}`"),
                    };
                    sql_invalid!(spec.index.is_some(), "field `{field}` already has an index");
                    spec.index = Some(index.index);
                }

                (
                    Statement::CreateTable {
                        table,
                        schema,
                        if_not_exists,
                    },
                    None,
                )
            }
            SqlStatement::CreateIndex(idx) => {
                let index = Index::from_sql(idx)?;
                let table = index.table;
                sql_invalid!("CREATE INDEX references unknown table `{table}`");
            }
            other => {
                let raw = matches!(other, SqlStatement::Query(_)).then(|| other.clone());
                (Statement::try_from(other)?, raw)
            }
        };
        out.push((stmt, raw));
    }

    Ok(out)
}

#[derive(Debug, Clone, PartialEq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum Statement {
    Select {
        /// Table name (`<collection>` OR `<collection>.<partition>`).
        table: Table,
        /// `topk_rs::Query` to execute.
        query: Query,
    },
    Count {
        /// Table name (`<collection>` OR `<collection>.<partition>`).
        table: Table,
        /// `topk_rs::Query` to execute.
        query: Query,
        /// Result column name (`_count` when unaliased).
        alias: String,
    },
    Insert {
        /// Table name (`<collection>` OR `<collection>.<partition>`).
        table: Table,
        /// Documents to insert.
        docs: Vec<Document>,
    },
    Update {
        /// Table name (`<collection>` OR `<collection>.<partition>`).
        table: Table,
        /// Documents to update.
        docs: Vec<Document>,
        /// Whether to fail the update if a document is missing.
        fail_on_missing: bool,
    },
    Delete {
        /// Table name (`<collection>` OR `<collection>.<partition>`).
        table: Table,
        /// Filter to apply to the documents to delete.
        filter: RowFilter,
    },
    DeletePartition {
        /// Table name (`<collection>.<partition>`).
        table: Table,
    },

    CreateTable {
        /// Table name (`<collection>`).
        table: Table,
        /// `topk_rs::FieldSpec` for each column.
        schema: HashMap<String, FieldSpec>,
        /// Silently ignore if the table already exists.
        if_not_exists: bool,
    },
    DropTable {
        /// Table name (`<collection>`).
        table: Table,
        /// Silently ignore if the table does not exist.
        if_exists: bool,
    },
    CreateIndex {
        /// Index to create.
        index: Index,
    },

    Explain {
        /// Statement to explain.
        stmt: Box<Statement>,
        /// Whether to include verbose information.
        verbose: bool,
    },
    Set {
        /// Variable to set (eg. `consistency_level`)
        variable: Variable,
        /// Value to set the variable to (eg. `'strong'`).
        value: Value,
    },
    Show {
        /// Variable to show (eg. `consistency_level`).
        variable: Variable,
    },

    /// `SELECT ... FROM information_schema.tables`
    InfoSchemaTables,
    /// `SELECT ... FROM information_schema.columns`
    InfoSchemaColumns {
        /// Table name (`<collection>`).
        table: String,
    },

    /// `BEGIN` statement is accepted but silently ignored
    Begin,
    /// `COMMIT` statement is accepted but silently ignored
    Commit,
    /// `ROLLBACK` statement is accepted but silently ignored
    Rollback,
    /// `DISCARD <anything>` statement is accepted but silently ignored
    Discard,
}

impl Statement {
    pub fn as_str(&self) -> &'static str {
        self.into()
    }

    pub fn table(&self) -> &Table {
        match self {
            Statement::Select { table, .. } | Statement::Count { table, .. } => table,
            Statement::Insert { table, .. } => table,
            Statement::Update { table, .. } => table,
            Statement::Delete { table, .. } => table,
            Statement::DeletePartition { table } => table,
            Statement::Explain { stmt, .. } => stmt.table(),
            Statement::CreateTable { table, .. } => table,
            Statement::DropTable { table, .. } => table,
            Statement::CreateIndex { index } => &index.table,
            Statement::Set { .. } | Statement::Show { .. } => {
                unreachable!("session commands have no table")
            }
            Statement::Begin | Statement::Commit | Statement::Rollback | Statement::Discard => {
                unreachable!("transaction commands have no table")
            }
            Statement::InfoSchemaTables | Statement::InfoSchemaColumns { .. } => {
                unreachable!("information_schema queries have no table")
            }
        }
    }
}

impl TryFrom<SqlStatement> for Statement {
    type Error = Error;

    fn try_from(stmt: SqlStatement) -> Result<Statement, Error> {
        match stmt {
            SqlStatement::StartTransaction { .. } => Ok(Statement::Begin),
            SqlStatement::Commit { .. } => Ok(Statement::Commit),
            SqlStatement::Rollback { .. } => Ok(Statement::Rollback),
            SqlStatement::SetVariable { .. } => set_variable::try_from_sql(stmt),
            SqlStatement::ShowVariable { .. } => show::try_from_sql(stmt),
            SqlStatement::Discard { .. } => Ok(Statement::Discard),

            SqlStatement::Query(q) => Statement::try_from(*q),
            SqlStatement::Insert(insert) => Statement::try_from(insert),
            SqlStatement::Update { .. } => update::try_from_sql(stmt),
            SqlStatement::Delete(delete) => Statement::try_from(delete),
            SqlStatement::Explain { .. } => explain::try_from_sql(stmt),
            SqlStatement::CreateTable(ct) => Statement::try_from(ct),
            SqlStatement::Drop { .. } => drop::try_from_sql(stmt),
            SqlStatement::CreateIndex(idx) => Statement::from_sql(idx),
            other => Err(Error::Unsupported(format!("statement: {other:?}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RowFilter {
    Ids(Vec<String>),
    Expr(LogicalExpr),
}

impl FromSql<SqlExpr> for RowFilter {
    fn from_sql(expr: SqlExpr) -> Result<Self, Error> {
        match expr {
            // Single ID filter: `_id = '<id>'` or `'<id>' = _id`.
            SqlExpr::BinaryOp {
                left,
                op: BinaryOperator::Eq,
                right,
            } if left.as_id().is_some() || right.as_id().is_some() => {
                let lit = if left.as_id().is_some() { right } else { left };
                let id = lit.as_string().ok_or_else(|| {
                    Error::Invalid(format!("expected a string for `_id`, got {lit:?}"))
                })?;
                Ok(RowFilter::Ids(vec![id]))
            }
            // List of IDs filter: `_id IN ('<id1>', '<id2>', …)`.
            SqlExpr::InList {
                expr,
                list,
                negated,
            } if expr.as_ident().is_some_and(|s| s == "_id") => {
                sql_unsupported!(negated, "`_id NOT IN (…)` is not supported");
                sql_invalid!(list.is_empty(), "`_id IN ()` with empty list");

                let ids = list
                    .into_iter()
                    .map(|expr| match expr.as_string() {
                        Some(id) => Ok(id),
                        None => sql_invalid!("`_id IN (…)` requires a list of strings"),
                    })
                    .collect::<Result<Vec<String>, Error>>()?;

                Ok(RowFilter::Ids(ids))
            }
            SqlExpr::Nested(inner) => Self::from_sql(*inner),
            other => Ok(RowFilter::Expr(LogicalExpr::from_sql(other)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_sql;
    use sqlparser::ast::Statement as SqlStatement;

    use super::*;

    fn parse_query(sql: &str) -> Statement {
        let SqlStatement::Query(q) = parse_sql(sql).unwrap().remove(0) else {
            panic!("expected query");
        };
        Statement::try_from(*q).unwrap()
    }

    #[test]
    fn count_parses_to_count_statement() {
        match parse_query("SELECT COUNT(*) FROM books") {
            Statement::Count { alias, .. } => assert_eq!(alias, "_count"),
            other => panic!("expected Count, got {other:?}"),
        }

        match parse_query("SELECT COUNT(*) AS n FROM books") {
            Statement::Count { alias, .. } => assert_eq!(alias, "n"),
            other => panic!("expected Count, got {other:?}"),
        }
    }

    #[test]
    fn standalone_create_index_reports_option_error_first() {
        let sql = "\
            CREATE INDEX ON books USING vector_index (embedding) \
            WITH (metric = 'cosine', typo = 'oops')\
        ";

        let err = aggregate_stmts(crate::parse_sql(sql).unwrap()).unwrap_err();

        assert_eq!(err.to_string(), "Invalid: unknown option `typo`");
    }
}
