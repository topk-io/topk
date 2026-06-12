use sqlparser::ast::{OneOrManyWithParens, Statement as SqlStatement};
use topk_rs::proto::v1::data::Value;

use crate::{Error, FromSql, Statement, sql_invalid, sql_unsupported, stmt::Variable};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    match stmt {
        SqlStatement::SetVariable {
            variables,
            mut value,
            ..
        } => {
            let name = match variables {
                OneOrManyWithParens::One(name) => name,
                OneOrManyWithParens::Many(_) => sql_unsupported!("multiple variables in SET"),
            };

            sql_invalid!(value.len() != 1, "SET requires exactly one value");
            let variable = Variable::from_sql(name)?;
            let value = Value::from_sql(value.remove(0))?;

            Ok(Statement::Set { variable, value })
        }
        _ => sql_unsupported!("not a SET statement"),
    }
}
