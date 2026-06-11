use sqlparser::ast::ObjectName;

use crate::{Error, FromSql, sql_invalid};

#[derive(Debug, Clone, PartialEq)]
pub enum Variable {
    ConsistencyLevel,
}

impl Variable {
    pub fn as_str(&self) -> &'static str {
        match self {
            Variable::ConsistencyLevel => "consistency_level",
        }
    }
}

impl FromSql<ObjectName> for Variable {
    fn from_sql(name: ObjectName) -> Result<Self, Error> {
        match name.to_string().as_str() {
            "consistency_level" => Ok(Variable::ConsistencyLevel),
            _ => sql_invalid!("unknown variable: {name}"),
        }
    }
}
