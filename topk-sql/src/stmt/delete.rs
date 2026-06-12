use sqlparser::ast::{Delete, FromTable, TableFactor};

use crate::{Error, FromSql, Statement, Table, sql_unsupported, stmt::RowFilter};

impl TryFrom<Delete> for Statement {
    type Error = Error;

    fn try_from(delete: Delete) -> Result<Statement, Error> {
        sql_unsupported!(!delete.tables.is_empty(), "multi-table DELETE");
        sql_unsupported!(delete.using.is_some(), "DELETE … USING");
        sql_unsupported!(delete.returning.is_some(), "DELETE … RETURNING");

        // Parse table
        let table = {
            let mut tables = match delete.from {
                FromTable::WithFromKeyword(t) | FromTable::WithoutKeyword(t) => t,
            };
            sql_unsupported!(tables.len() != 1, "DELETE requires exactly one table");
            let table = tables.remove(0);
            sql_unsupported!(!table.joins.is_empty(), "DELETE with JOIN");

            let table = match table.relation {
                TableFactor::Table {
                    name, args: None, ..
                } => Table::new(name)?,
                TableFactor::Table { .. } => sql_unsupported!("table function in DELETE"),
                _ => sql_unsupported!("non-table target in DELETE"),
            };

            table
        };

        // `DELETE FROM <collection>$<partition>` with no `WHERE` clause maps to `DeletePartition`.
        if matches!(table, Table::Partition(_, _)) && delete.selection.is_none() {
            return Ok(Statement::DeletePartition { table });
        }

        // `DELETE FROM <collection>` requires a `WHERE` clause.
        let r#where = delete.selection.ok_or_else(|| {
            Error::Invalid("DELETE without a WHERE clause is not allowed".to_string())
        })?;
        let filter = RowFilter::from_sql(r#where.clone())?;

        Ok(Statement::Delete { table, filter })
    }
}
