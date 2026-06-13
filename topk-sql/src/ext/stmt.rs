use std::ops::ControlFlow;

use sqlparser::ast::{
    AssignmentTarget, Expr as SqlExpr, FromTable, SelectItem, SetExpr, Statement as SqlStatement,
    Value as SqlValue, visit_expressions,
};

use super::{ObjectNameExt, SqlExprExt, TableFactorExt};

pub trait SqlStatementExt {
    fn projection(&self) -> Option<&[SelectItem]>;

    /// Count the number of placeholders in the statement.
    fn count_placeholders(&self) -> usize;

    /// Get the table reference (schema, name) from the statement, if present.
    fn table_ref(&self) -> Option<(Option<&str>, &str)>;

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
            if let SqlExpr::Value(SqlValue::Placeholder(_)) = expr {
                count += 1;
            }
            ControlFlow::Continue(())
        });
        count
    }

    fn table_ref(&self) -> Option<(Option<&str>, &str)> {
        match self {
            SqlStatement::Query(q) => {
                if let SetExpr::Select(s) = q.body.as_ref() {
                    s.from.first().and_then(|twj| twj.relation.as_table_ref())
                } else {
                    None
                }
            }
            SqlStatement::Update { table, .. } => table.relation.as_table_ref(),
            SqlStatement::Insert(i) => i.table_name.as_table_ref(),
            SqlStatement::Delete(d) => {
                let tables = match &d.from {
                    FromTable::WithFromKeyword(t) | FromTable::WithoutKeyword(t) => t,
                };
                tables.first().and_then(|twj| twj.relation.as_table_ref())
            }
            _ => None,
        }
    }

    fn assignment_placeholders(&self) -> Vec<(usize, &str)> {
        match self {
            SqlStatement::Update { assignments, .. } => assignments
                .iter()
                .filter_map(|a| {
                    if let AssignmentTarget::ColumnName(name) = &a.target {
                        let col = name.0.last()?.value.as_str();
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

    #[rstest]
    #[case("SELECT * FROM books", Some((None, "books")))]
    #[case("SELECT * FROM public.books", Some((Some("public"), "books")))]
    #[case("SELECT * FROM myschema.books", Some((Some("myschema"), "books")))]
    #[case("SELECT 1", None)]
    #[case("SELECT * FROM (SELECT 1) sub", None)]
    #[case("UPDATE books SET title = $1 WHERE _id = $2", Some((None, "books")))]
    #[case("UPDATE public.books SET title = $1", Some((Some("public"), "books")))]
    #[case("INSERT INTO books (_id, title) VALUES ('1', 'a')", Some((None, "books")))]
    #[case("DELETE FROM books WHERE _id = '1'", Some((None, "books")))]
    fn table_ref(#[case] sql: &str, #[case] expected: Option<(Option<&str>, &str)>) {
        assert_eq!(parse_one(sql).table_ref(), expected);
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
