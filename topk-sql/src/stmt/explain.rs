use sqlparser::ast::Statement as SqlStatement;

use crate::{Error, Statement, sql_unsupported};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    match stmt {
        SqlStatement::Explain {
            analyze,
            verbose,
            query_plan,
            statement,
            format,
            options,
            ..
        } => {
            sql_unsupported!(analyze, "EXPLAIN with ANALYZE");
            sql_unsupported!(query_plan, "EXPLAIN with QUERY PLAN");
            sql_unsupported!(format.is_some(), "EXPLAIN with FORMAT");
            sql_unsupported!(options.is_some(), "EXPLAIN with OPTIONS");

            Ok(Statement::Explain {
                stmt: Box::new(Statement::try_from(*statement)?),
                verbose,
            })
        }
        _ => sql_unsupported!("not an EXPLAIN statement"),
    }
}
