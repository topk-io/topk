use std::ops::ControlFlow;

use sqlparser::ast::{
    Expr as SqlExpr, SetExpr, Statement as SqlStatement, TableFactor, Value as SqlValue,
    visit_expressions,
};

pub trait SqlStatementExt {
    /// Count the number of placeholders in the statement.
    fn count_placeholders(&self) -> usize;

    /// Get the table name from the statement.
    fn table_name(&self) -> Option<String>;
}

impl SqlStatementExt for SqlStatement {
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

    fn table_name(&self) -> Option<String> {
        if let SqlStatement::Query(q) = self {
            if let SetExpr::Select(s) = q.body.as_ref() {
                if let Some(twj) = s.from.first() {
                    if let TableFactor::Table { name, .. } = &twj.relation {
                        return Some(name.0.last()?.value.clone());
                    }
                }
            }
        }

        None
    }
}
