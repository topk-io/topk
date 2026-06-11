use std::collections::HashMap;

use sqlparser::ast::{AssignmentTarget, Statement as SqlStatement, TableFactor};
use topk_rs::doc;
use topk_rs::proto::v1::data::Value;

use crate::{Error, FromSql, Statement, Table, sql_invalid, sql_unsupported, stmt::RowFilter};

pub fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    match stmt {
        SqlStatement::Update {
            table,
            assignments,
            from,
            returning,
            selection,
        } => {
            sql_unsupported!(from.is_some(), "UPDATE … FROM");
            sql_unsupported!(returning.is_some(), "UPDATE … RETURNING");
            sql_unsupported!(!table.joins.is_empty(), "UPDATE with JOIN");
            sql_invalid!(assignments.is_empty(), "UPDATE requires at least one SET");

            let table = match table.relation {
                TableFactor::Table {
                    name, args: None, ..
                } => Table::new(name)?,
                TableFactor::Table { .. } => sql_unsupported!("table function in UPDATE"),
                _ => sql_unsupported!("non-table target in UPDATE"),
            };

            let mut updates = HashMap::with_capacity(assignments.len());
            for assignment in assignments {
                let field = match assignment.target {
                    AssignmentTarget::ColumnName(name) => {
                        let name = name.to_string();
                        sql_invalid!(&name == "_id", "cannot UPDATE the `_id` field");
                        Result::<_, crate::Error>::Ok(name)
                    }
                    AssignmentTarget::Tuple(_) => {
                        sql_unsupported!("tuple assignment `(a, b) = …`")
                    }
                }?;

                let value = Value::from_sql(assignment.value)?;
                if updates.insert(field.clone(), value).is_some() {
                    sql_invalid!("field `{field}` assigned more than once");
                }
            }

            // Parse WHERE clause
            let filter = selection.map(RowFilter::from_sql).transpose()?;
            let ids = match filter {
                Some(RowFilter::Ids(ids)) => ids,
                _ => sql_invalid!("UPDATE requires a `WHERE _id = …` or `WHERE _id IN (…)` clause"),
            };

            let docs = ids
                .into_iter()
                .map(|id| {
                    let mut d = doc!("_id" => id);
                    for (field, value) in updates.iter() {
                        d.fields.insert(field.clone(), value.clone());
                    }
                    d
                })
                .collect();

            Ok(Statement::Update {
                table,
                docs,
                fail_on_missing: false,
            })
        }
        _ => sql_unsupported!("not a UPDATE statement"),
    }
}
