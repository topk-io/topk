use std::collections::HashMap;

use sqlparser::ast::{AssignmentTarget, TableFactor, Update};
use topk_rs::doc;
use topk_rs::proto::v1::data::Value;

use crate::{Error, FromSql, Statement, Table, sql_invalid, sql_unsupported, stmt::RowFilter};

impl TryFrom<Update> for Statement {
    type Error = Error;

    fn try_from(stmt: Update) -> Result<Statement, Error> {
        sql_unsupported!(stmt.from.is_some(), "UPDATE … FROM");
        sql_unsupported!(stmt.returning.is_some(), "UPDATE … RETURNING");
        sql_unsupported!(!stmt.table.joins.is_empty(), "UPDATE with JOIN");
        sql_invalid!(
            stmt.assignments.is_empty(),
            "UPDATE requires at least one SET"
        );

        let table = match stmt.table.relation {
            TableFactor::Table {
                name, args: None, ..
            } => Table::new(name)?,
            TableFactor::Table { .. } => sql_unsupported!("table function in UPDATE"),
            _ => sql_unsupported!("non-table target in UPDATE"),
        };

        let mut updates = HashMap::with_capacity(stmt.assignments.len());
        for assignment in stmt.assignments {
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

        let filter = stmt.selection.map(RowFilter::from_sql).transpose()?;
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
}
