use std::ops::ControlFlow;

use sqlparser::ast::{
    AssignmentTarget, Expr as SqlExpr, FromTable, ObjectName, SelectItem, SetExpr,
    Statement as SqlStatement, TableObject, Value as SqlValue, visit_expressions, visit_relations,
};

use super::{SqlExprExt, TableFactorExt};
use crate::{Error, Table};

pub trait SqlStatementExt {
    fn projection(&self) -> Option<&[SelectItem]>;

    /// Count the number of placeholders in the statement.
    fn count_placeholders(&self) -> usize;

    /// Get the table name from the statement, if present.
    fn table(&self) -> Result<Option<Table>, Error>;

    /// For UPDATE statements, return `(placeholder_index, column_name)` pairs
    /// for each `SET col = $N` assignment whose value is a direct placeholder.
    fn assignment_placeholders(&self) -> Vec<(usize, &str)>;

    /// True if any relation in the statement satisfies `pred`.
    fn any_relation(&self, pred: impl FnMut(&ObjectName) -> bool) -> bool;
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

    fn table(&self) -> Result<Option<Table>, Error> {
        let name = match self {
            SqlStatement::Query(q) => match q.body.as_ref() {
                SetExpr::Select(s) => {
                    match s.from.first().and_then(|from| from.relation.table_name()) {
                        Some(name) => name,
                        None => return Ok(None),
                    }
                }
                _ => return Ok(None),
            },
            SqlStatement::Update(u) => match u.table.relation.table_name() {
                Some(name) => name,
                None => return Ok(None),
            },
            SqlStatement::Insert(i) => match &i.table {
                TableObject::TableName(name) => name,
                TableObject::TableFunction(_) => return Ok(None),
            },
            SqlStatement::Delete(d) => {
                let tables = match &d.from {
                    FromTable::WithFromKeyword(t) | FromTable::WithoutKeyword(t) => t,
                };
                match tables.first().and_then(|from| from.relation.table_name()) {
                    Some(name) => name,
                    None => return Ok(None),
                }
            }
            _ => return Ok(None),
        };

        let table = Table::new(name.clone())?;

        Ok(Some(table))
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

    fn any_relation(&self, mut pred: impl FnMut(&ObjectName) -> bool) -> bool {
        visit_relations(self, |name| {
            if pred(name) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_break()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::SqlStatementExt;
    use crate::{Error, ObjectNameExt, Table, parse_sql};
    use sqlparser::ast::Statement as SqlStatement;

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
        assert_eq!(parse_one(sql).table().unwrap().unwrap(), expected);
    }

    #[rstest]
    #[case("SELECT 1")]
    #[case("SELECT * FROM (SELECT 1) sub")]
    fn table_none(#[case] sql: &str) {
        assert_eq!(parse_one(sql).table().unwrap(), None);
    }

    #[rstest]
    #[case(
        "SELECT * FROM myschema.books",
        "unknown schema 'myschema'; only 'public' is supported"
    )]
    #[case(
        "UPDATE information_schema.books SET title = $1",
        "unknown schema 'information_schema'; only 'public' is supported"
    )]
    fn table_invalid_name(#[case] sql: &str, #[case] message: &str) {
        match parse_one(sql).table().unwrap_err() {
            Error::Invalid(msg) => assert_eq!(msg, message),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[rstest]
    #[case("UPDATE books SET title = $1 WHERE _id = $2", vec![(0, "title")])]
    #[case("UPDATE books SET title = $1, score = $2", vec![(0, "title"), (1, "score")])]
    #[case("UPDATE books SET title = 'x'", vec![])]
    #[case("SELECT * FROM books", vec![])]
    fn assignment_placeholders(#[case] sql: &str, #[case] expected: Vec<(usize, &str)>) {
        assert_eq!(parse_one(sql).assignment_placeholders(), expected);
    }

    #[rstest]
    #[case("SELECT * FROM pg_catalog.pg_class", true)]
    #[case("SELECT * FROM books", false)]
    #[case("SELECT * FROM books JOIN information_schema.tables ON true", true)]
    fn any_relation(#[case] sql: &str, #[case] expected: bool) {
        assert_eq!(
            parse_one(sql).any_relation(|name| {
                name.schema().is_some_and(|s| {
                    matches!(
                        s.to_ascii_lowercase().as_str(),
                        "pg_catalog" | "information_schema"
                    )
                })
            }),
            expected
        );
    }
}
