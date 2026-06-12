use std::ops::ControlFlow;

use sqlparser::ast::{
    Expr as SqlExpr, SelectItem, SetExpr, Statement as SqlStatement, TableFactor,
    Value as SqlValue, visit_expressions,
};

pub trait SqlStatementExt {
    fn projection(&self) -> Option<&[SelectItem]>;

    /// Count the number of placeholders in the statement.
    fn count_placeholders(&self) -> usize;

    /// Get the table reference (schema, name) from the FROM clause, if present.
    fn table_ref(&self) -> Option<(Option<&str>, &str)>;
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
        if let SqlStatement::Query(q) = self {
            if let SetExpr::Select(s) = q.body.as_ref() {
                if let Some(twj) = s.from.first() {
                    if let TableFactor::Table { name, .. } = &twj.relation {
                        let table = name.0.last()?.value.as_str();
                        let schema =
                            (name.0.len() > 1).then(|| name.0[name.0.len() - 2].value.as_str());
                        return Some((schema, table));
                    }
                }
            }
        }
        None
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
    fn table_ref(#[case] sql: &str, #[case] expected: Option<(Option<&str>, &str)>) {
        assert_eq!(parse_one(sql).table_ref(), expected);
    }
}
