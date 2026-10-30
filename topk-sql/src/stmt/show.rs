use sqlparser::ast::{Statement as SqlStatement, Value as SqlValue};

use crate::{sql_invalid, sql_unsupported, Error, SqlExprExt, SqlStatementExt, Statement, Table};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    let Some(name) = stmt.show_partitions_table() else {
        sql_unsupported!("statement: {stmt:?}");
    };

    let table = Table::new(name.clone())?;
    sql_invalid!(
        !matches!(table, Table::Collection(_)),
        "SHOW PARTITIONS requires a collection name"
    );
    // `TopKDialect` only writes a string here; an empty prefix matches every partition
    let prefix = match &stmt {
        SqlStatement::ShowObjects(show) => match &show.show_options.starts_with {
            Some(SqlValue::SingleQuotedString(prefix)) if !prefix.is_empty() => {
                Some(prefix.clone())
            }
            _ => None,
        },
        _ => None,
    };
    let limit = stmt
        .limit()
        .map(|expr| {
            expr.as_u64()
                .ok_or_else(|| Error::Invalid("LIMIT must be a non-negative integer".to_string()))
        })
        .transpose()?;

    Ok(Statement::ShowPartitions {
        table,
        prefix,
        limit,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{convert_sql, parse_sql, Error, Statement, Table};

    fn convert(sql: &str) -> Result<Statement, Error> {
        convert_sql(parse_sql(sql)?).map(|mut batch| batch.remove(0).0)
    }

    #[rstest]
    #[case("SHOW PARTITIONS FROM TABLE books", None, None)]
    #[case("show partitions from table books", None, None)]
    #[case("SHOW PARTITIONS FROM TABLE public.books", None, None)]
    #[case("SHOW PARTITIONS FROM TABLE \"books\"", None, None)]
    #[case("SHOW PARTITIONS FROM TABLE books LIMIT 10", None, Some(10))]
    #[case(
        "SHOW PARTITIONS FROM TABLE books STARTS WITH 'acme'",
        Some("acme"),
        None
    )]
    #[case(
        "SHOW PARTITIONS FROM TABLE books STARTS WITH 'acme' LIMIT 10",
        Some("acme"),
        Some(10)
    )]
    #[case("SHOW PARTITIONS FROM TABLE books STARTS WITH ''", None, None)]
    #[case("SHOW PARTITIONS FROM TABLE books;", None, None)]
    fn show_partitions(
        #[case] sql: &str,
        #[case] prefix: Option<&str>,
        #[case] limit: Option<u64>,
    ) {
        assert_eq!(
            convert(sql).unwrap(),
            Statement::ShowPartitions {
                table: Table::Collection("books".into()),
                prefix: prefix.map(str::to_string),
                limit,
            }
        );
    }

    #[rstest]
    #[case::partition("SHOW PARTITIONS FROM TABLE books$p1")]
    #[case::schema("SHOW PARTITIONS FROM TABLE other.books")]
    #[case::no_table("SHOW PARTITIONS FROM TABLE")]
    #[case::no_from("SHOW PARTITIONS books")]
    #[case::space_for_dot("SHOW PARTITIONS FROM TABLE public books")]
    #[case::trailing("SHOW PARTITIONS FROM TABLE books (")]
    #[case::negative_limit("SHOW PARTITIONS FROM TABLE books LIMIT -1")]
    #[case::text_limit("SHOW PARTITIONS FROM TABLE books LIMIT 'ten'")]
    #[case::prefix_not_string("SHOW PARTITIONS FROM TABLE books STARTS WITH acme")]
    #[case::prefix_placeholder("SHOW PARTITIONS FROM TABLE books STARTS WITH $1")]
    #[case::limit_before_prefix("SHOW PARTITIONS FROM TABLE books LIMIT 10 STARTS WITH 'acme'")]
    #[case::variable("SHOW transaction_isolation")]
    fn show_rejected(#[case] sql: &str) {
        assert!(convert(sql).is_err(), "{sql}");
    }
}
