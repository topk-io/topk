use sqlparser::ast::{DescribeAlias, Statement as SqlStatement};

use crate::{sql_invalid, sql_unsupported, Error, Statement, Table};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    let SqlStatement::ExplainTable {
        describe_alias,
        table_name,
        ..
    } = stmt
    else {
        sql_unsupported!("statement: {stmt}");
    };
    sql_unsupported!(
        describe_alias == DescribeAlias::Explain,
        "EXPLAIN <table>; use DESCRIBE <table>"
    );

    let table = Table::new(table_name)?;
    sql_invalid!(
        !matches!(table, Table::Collection(_)),
        "DESCRIBE requires a collection name"
    );

    Ok(Statement::Describe { table })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{convert_sql, parse_sql, Error, Statement, Table};

    fn convert(sql: &str) -> Result<Statement, Error> {
        convert_sql(parse_sql(sql)?).map(|mut batch| batch.remove(0).0)
    }

    #[rstest]
    #[case("DESCRIBE books", "books")]
    #[case("DESC books", "books")]
    #[case("DESCRIBE public.books", "books")]
    #[case("DESCRIBE \"logs-2026.07.29\"", "logs-2026.07.29")]
    #[case("DESCRIBE TABLE books", "books")]
    #[case("DESC TABLE books", "books")]
    #[case("DESCRIBE COLLECTION books", "books")]
    #[case("describe collection public.books", "books")]
    fn describe(#[case] sql: &str, #[case] collection: &str) {
        assert_eq!(
            convert(sql).unwrap(),
            Statement::Describe {
                table: Table::Collection(collection.into()),
            }
        );
    }

    #[rstest]
    #[case::explain("EXPLAIN books")]
    #[case::partition("DESCRIBE books$p1")]
    #[case::extended("DESCRIBE EXTENDED books")]
    #[case::statement("DESCRIBE SELECT title FROM books")]
    #[case::desc_statement("DESC SELECT title FROM books")]
    #[case::table_keyword_only("DESCRIBE TABLE")]
    #[case::collection_partition("DESCRIBE COLLECTION books$p1")]
    #[case::trailing("DESCRIBE TABLE books extra")]
    fn describe_rejects(#[case] sql: &str) {
        assert!(
            parse_sql(sql).and_then(convert_sql).is_err(),
            "{sql} should be rejected"
        );
    }
}
