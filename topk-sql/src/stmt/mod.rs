use std::collections::HashMap;

use sqlparser::ast::{BinaryOperator, Expr as SqlExpr, Statement as SqlStatement};
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

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Select {
        table: Table,
        query: Query,
    },
    Insert {
        table: Table,
        docs: Vec<Document>,
    },
    Update {
        table: Table,
        docs: Vec<Document>,
        fail_on_missing: bool,
    },
    Delete {
        table: Table,
        filter: RowFilter,
    },
    DeletePartition {
        table: Table,
    },

    CreateTable {
        table: Table,
        schema: HashMap<String, FieldSpec>,
        if_not_exists: bool,
    },
    DropTable {
        table: Table,
        if_exists: bool,
    },
    CreateIndex {
        index: Index,
    },

    Explain {
        stmt: Box<Statement>,
        verbose: bool,
    },
    Set {
        variable: Variable,
        value: Value,
    },
    Show {
        variable: Variable,
    },

    InfoSchemaTables,
    InfoSchemaColumns {
        table: String,
    },

    Begin,
    Commit,
    Rollback,
    Discard,
}

impl Statement {
    pub fn table(&self) -> &Table {
        match self {
            Statement::Select { table, .. } => table,
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
    use super::*;

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
