use sqlparser::ast::{ObjectType, Statement as SqlStatement};

use crate::{sql_invalid, sql_unsupported, Error, Statement, Table};

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
            table,
            ..
        } => {
            sql_unsupported!(
                !matches!(object_type, ObjectType::Table | ObjectType::Index),
                "DROP {object_type}"
            );
            sql_unsupported!(temporary, "DROP TEMPORARY TABLE");
            sql_unsupported!(cascade, "DROP {object_type} … CASCADE");
            sql_unsupported!(restrict, "DROP {object_type} … RESTRICT");
            sql_unsupported!(purge, "DROP {object_type} … PURGE");

            if object_type == ObjectType::Index {
                // an index is a property of a column, so the collection names it, not a lookup
                let table = match table {
                    Some(table) => Table::new(table)?,
                    None => sql_invalid!("DROP INDEX requires `ON <collection>`"),
                };
                sql_invalid!(names.len() != 1, "DROP INDEX must name exactly one index");
                let name = match names.remove(0).0.last().and_then(|part| part.as_ident()) {
                    Some(ident) => ident.value.clone(),
                    None => sql_invalid!("index name must be an identifier"),
                };

                return Ok(Statement::DropIndex {
                    table,
                    name,
                    if_exists,
                });
            }

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
