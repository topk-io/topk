use std::ops::ControlFlow;

use sqlparser::ast::{
    AssignmentTarget, Expr as SqlExpr, FromTable, SelectItem, SetExpr, Statement as SqlStatement,
    TableObject, Value as SqlValue, visit_expressions,
};

use super::{SqlExprExt, TableFactorExt};
use crate::Table;

pub trait SqlStatementExt {
    fn projection(&self) -> Option<&[SelectItem]>;

    /// Count the number of placeholders in the statement.
    fn count_placeholders(&self) -> usize;

    /// Get the table name from the statement, if present.
    fn table(&self) -> Option<Table>;

    /// For UPDATE statements, return `(placeholder_index, column_name)` pairs
    /// for each `SET col = $N` assignment whose value is a direct placeholder.
    fn assignment_placeholders(&self) -> Vec<(usize, &str)>;
}

impl SqlStatementExt for SqlStatement {
    fn projection(&self) -> Option<&[SelectItem]> {
        match self {
            SqlStatement::Query(q) => match q.body.as_ref() {
                SetExpr::Select(s) => Some(&s.projection),
                _ => None,
            },
            _ => None,
        }
    }

    fn count_placeholders(&self) -> usize {
        let mut count = 0;
        let _: ControlFlow<()> = visit_expressions(self, |expr| {
            if matches!(expr, SqlExpr::Value(v) if matches!(v.value, SqlValue::Placeholder(_))) {
                count += 1;
            }
            ControlFlow::Continue(())
        });
        count
    }

    fn table(&self) -> Option<Table> {
        let name = match self {
            SqlStatement::Query(q) => match q.body.as_ref() {
                SetExpr::Select(s) => s.from.first()?.relation.table_name()?,
                _ => return None,
            },
            SqlStatement::Update(u) => u.table.relation.table_name()?,
            SqlStatement::Insert(i) => match &i.table {
                TableObject::TableName(name) => name,
                TableObject::TableFunction(_) => return None,
            },
            SqlStatement::Delete(d) => {
                let tables = match &d.from {
                    FromTable::WithFromKeyword(t) | FromTable::WithoutKeyword(t) => t,
                };
                tables.first()?.relation.table_name()?
            }
            _ => return None,
        };

        Table::new(name.clone()).ok()
    }

    fn assignment_placeholders(&self) -> Vec<(usize, &str)> {
        match self {
            SqlStatement::Update(u) => u
                .assignments
                .iter()
                .filter_map(|a| {
                    if let AssignmentTarget::ColumnName(name) = &a.target {
                        let col = name.0.last()?.as_ident()?.value.as_str();
                        if let Some(idx) = a.value.as_placeholder() {
                            return Some((idx, col));
                        }
                    }
                    None
                })
                .collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::parse_sql;

    fn parse_one(sql: &str) -> SqlStatement {
        parse_sql(sql).unwrap().pop().unwrap()
    }

    // Name parsing (schema, `$partition`) is covered by `Table::new`'s own tests; here we only
    // check that each statement shape extracts the table from the right clause.
    #[rstest]
    #[case("SELECT * FROM books", Table::Collection("books".into()))]
    #[case("UPDATE books SET title = $1 WHERE _id = $2", Table::Collection("books".into()))]
    #[case("INSERT INTO books (_id, title) VALUES ('1', 'a')", Table::Collection("books".into()))]
    #[case("DELETE FROM books WHERE _id = '1'", Table::Collection("books".into()))]
    fn table(#[case] sql: &str, #[case] expected: Table) {
        assert_eq!(parse_one(sql).table().unwrap(), expected);
    }

    #[rstest]
    #[case("SELECT 1")]
    #[case("SELECT * FROM (SELECT 1) sub")]
    fn table_none(#[case] sql: &str) {
        assert_eq!(parse_one(sql).table(), None);
    }

    #[rstest]
    #[case("UPDATE books SET title = $1 WHERE _id = $2", vec![(0, "title")])]
    #[case("UPDATE books SET title = $1, score = $2", vec![(0, "title"), (1, "score")])]
    #[case("UPDATE books SET title = 'x'", vec![])]
    #[case("SELECT * FROM books", vec![])]
    fn assignment_placeholders(#[case] sql: &str, #[case] expected: Vec<(usize, &str)>) {
        assert_eq!(parse_one(sql).assignment_placeholders(), expected);
    }
}
