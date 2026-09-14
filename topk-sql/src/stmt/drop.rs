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
                let names = names
                    .into_iter()
                    .map(
                        |name| match name.0.last().and_then(|part| part.as_ident()) {
                            Some(ident) => Ok(ident.value.clone()),
                            None => sql_invalid!("index name must be an identifier"),
                        },
                    )
                    .collect::<Result<Vec<String>, Error>>()?;

                return Ok(Statement::DropIndex { names, if_exists });
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
