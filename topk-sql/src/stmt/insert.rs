use std::collections::HashMap;

use sqlparser::ast::{Insert, SetExpr};
use topk_rs::proto::v1::data::{Document, Value};

use crate::{Error, FromSql, Statement, Table, sql_invalid, sql_unsupported};

impl TryFrom<Insert> for Statement {
    type Error = Error;

    fn try_from(insert: Insert) -> Result<Statement, Error> {
        sql_unsupported!(insert.on.is_some(), "INSERT … ON CONFLICT");
        sql_unsupported!(insert.returning.is_some(), "INSERT … RETURNING");

        let table = Table::new(insert.table_name)?;

        // Parse query source.
        let source = insert
            .source
            .ok_or_else(|| Error::Invalid("INSERT requires VALUES".to_string()))?;

        sql_unsupported!(source.with.is_some(), "WITH clause");
        sql_unsupported!(source.order_by.is_some(), "INSERT ... ORDER BY");
        sql_unsupported!(source.limit.is_some(), "INSERT ... LIMIT");
        sql_unsupported!(source.offset.is_some(), "INSERT ... OFFSET");
        sql_unsupported!(source.fetch.is_some(), "INSERT ... FETCH");
        sql_unsupported!(
            !source.locks.is_empty(),
            "INSERT ... FOR {{ UPDATE | SHARE }}"
        );

        sql_invalid!(
            insert.columns.is_empty(),
            "INSERT requires an explicit column list"
        );

        // Parse values.
        let rows = match *source.body {
            SetExpr::Values(values) => values.rows,
            SetExpr::Select(_) => sql_unsupported!("INSERT … SELECT"),
            _ => sql_unsupported!("INSERT must be used with VALUES"),
        };
        if !insert.columns.iter().any(|c| c.value.as_str() == "_id") {
            sql_invalid!("INSERT column list must include `_id`");
        }

        // Check for duplicate columns.
        for (i, c) in insert.columns.iter().enumerate() {
            if insert.columns[..i].contains(c) {
                return Err(Error::Invalid(format!(
                    "column `{c}` specified more than once"
                )));
            }
        }

        // Parse rows.
        let mut docs = Vec::with_capacity(rows.len());
        for row in rows {
            // Check row length.
            if row.len() != insert.columns.len() {
                return Err(Error::Invalid(format!(
                    "VALUES row has {} entries, expected {}",
                    row.len(),
                    insert.columns.len()
                )));
            }

            // Parse row fields.
            let mut fields = HashMap::with_capacity(row.len());
            for (col, expr) in insert.columns.iter().zip(row) {
                fields.insert(col.value.to_string(), Value::from_sql(expr)?);
            }
            docs.push(Document { fields });
        }

        Ok(Statement::Insert { table, docs })
    }
}
