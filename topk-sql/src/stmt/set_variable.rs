use sqlparser::ast::Set;
use topk_rs::proto::v1::data::Value;

use crate::{sql_invalid, sql_unsupported, stmt::Variable, Error, FromSql, Statement};

impl TryFrom<Set> for Statement {
    type Error = Error;

    fn try_from(set: Set) -> Result<Statement, Error> {
        match set {
            Set::SingleAssignment {
                variable,
                mut values,
                ..
            } => {
                sql_invalid!(values.len() != 1, "SET requires exactly one value");
                let variable = Variable::from_sql(variable)?;
                let value = Value::from_sql(values.remove(0))?;
                Ok(Statement::Set { variable, value })
            }
            _ => sql_unsupported!("invalid SET statement"),
        }
    }
}
