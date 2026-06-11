use sqlparser::ast::{ObjectName, Statement as SqlStatement};

use crate::{Error, FromSql, Statement, sql_unsupported, stmt::Variable};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    match stmt {
        SqlStatement::ShowVariable { variable } => Ok(Statement::Show {
            variable: Variable::from_sql(ObjectName(variable))?,
        }),
        _ => sql_unsupported!("SHOW requires a variable name"),
    }
}
