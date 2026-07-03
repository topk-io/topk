use sqlparser::ast::{ObjectName, ObjectNamePart, Statement as SqlStatement};

use crate::{Error, FromSql, Statement, sql_unsupported, stmt::Variable};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    match stmt {
        SqlStatement::ShowVariable { variable } => Ok(Statement::Show {
            variable: Variable::from_sql(ObjectName(
                variable
                    .into_iter()
                    .map(ObjectNamePart::Identifier)
                    .collect(),
            ))?,
        }),
        _ => sql_unsupported!("SHOW requires a variable name"),
    }
}
