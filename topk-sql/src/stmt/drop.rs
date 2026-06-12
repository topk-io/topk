use sqlparser::ast::{ObjectType, Statement as SqlStatement};

use crate::{Error, Statement, Table, sql_invalid, sql_unsupported};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    match stmt {
        SqlStatement::Drop {
            object_type,
            if_exists,
            mut names,
            cascade,
            restrict,
            purge,
            temporary,
        } => {
            sql_unsupported!(object_type != ObjectType::Table, "DROP {object_type}");
            sql_unsupported!(temporary, "DROP TEMPORARY TABLE");
            sql_unsupported!(cascade, "DROP TABLE … CASCADE");
            sql_unsupported!(restrict, "DROP TABLE … RESTRICT");
            sql_unsupported!(purge, "DROP TABLE … PURGE");
            sql_invalid!(names.len() != 1, "DROP TABLE must name exactly one table");

            let table = Table::new(names.remove(0))?;
            sql_invalid!(
                !matches!(table, Table::Collection(_)),
                "DROP TABLE requires a collection name"
            );

            Ok(Statement::DropTable { table, if_exists })
        }
        _ => sql_unsupported!("not a DROP statement"),
    }
}
