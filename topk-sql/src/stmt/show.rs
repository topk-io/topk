use sqlparser::ast::{Statement as SqlStatement, Value as SqlValue};

use crate::{sql_invalid, sql_unsupported, Error, SqlExprExt, SqlStatementExt, Statement, Table};

pub(crate) fn try_from_sql(stmt: SqlStatement) -> Result<Statement, Error> {
    let SqlStatement::ShowObjects(show) = &stmt else {
        sql_unsupported!("statement: {stmt}");
    };

    let prefix = match &show.show_options.starts_with {
        None => None,
        Some(SqlValue::SingleQuotedString(prefix)) => (!prefix.is_empty()).then(|| prefix.clone()),
        Some(other) => sql_invalid!("STARTS WITH expects a string, found: {other}"),
    };
    let limit = show
        .show_options
        .limit
        .as_ref()
        .map(|expr| {
            expr.as_u64()
                .ok_or_else(|| Error::Invalid("LIMIT must be a non-negative integer".to_string()))
        })
        .transpose()?;

    // `SHOW PARTITIONS FROM TABLE <name>` has a table; `SHOW TABLES` does not
    match stmt.table()? {
        Some(table) => {
            sql_invalid!(
                !matches!(table, Table::Collection(_)),
                "SHOW PARTITIONS requires a collection name"
            );

            Ok(Statement::ShowPartitions {
                table,
                prefix,
                limit,
            })
        }
        None => Ok(Statement::ShowTables { prefix, limit }),
    }
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
    #[case("SHOW PARTITIONS FROM COLLECTION books", None, None)]
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
    #[case("SHOW TABLES", None, None)]
    #[case("show tables", None, None)]
    #[case("SHOW TABLES LIMIT 10", None, Some(10))]
    #[case("SHOW TABLES STARTS WITH 'acme'", Some("acme"), None)]
    #[case("SHOW TABLES STARTS WITH 'acme' LIMIT 10", Some("acme"), Some(10))]
    #[case("SHOW TABLES STARTS WITH ''", None, None)]
    #[case("SHOW TABLES;", None, None)]
    #[case("SHOW COLLECTIONS", None, None)]
    #[case("show collections starts with 'acme' limit 10", Some("acme"), Some(10))]
    fn show_tables(#[case] sql: &str, #[case] prefix: Option<&str>, #[case] limit: Option<u64>) {
        assert_eq!(
            convert(sql).unwrap(),
            Statement::ShowTables {
                prefix: prefix.map(str::to_string),
                limit,
            }
        );
    }

    #[rstest]
    #[case::tables_from("SHOW TABLES FROM SCHEMA public")]
    #[case::tables_like("SHOW TABLES LIKE 'acme%'")]
    #[case::tables_full("SHOW FULL TABLES")]
    #[case::tables_trailing("SHOW TABLES books")]
    #[case::tables_negative_limit("SHOW TABLES LIMIT -1")]
    #[case::tables_prefix_not_string("SHOW TABLES STARTS WITH acme")]
    #[case::tables_limit_before_prefix("SHOW TABLES LIMIT 10 STARTS WITH 'acme'")]
    #[case::quoted_tables("SHOW \"tables\"")]
    #[case::quoted_collections("SHOW \"collections\"")]
    #[case::partition("SHOW PARTITIONS FROM TABLE books$p1")]
    #[case::schema("SHOW PARTITIONS FROM TABLE other.books")]
    #[case::no_table("SHOW PARTITIONS FROM TABLE")]
    #[case::no_from("SHOW PARTITIONS books")]
    #[case::no_kind("SHOW PARTITIONS FROM books")]
    #[case::space_for_dot("SHOW PARTITIONS FROM TABLE public books")]
    #[case::trailing("SHOW PARTITIONS FROM TABLE books (")]
    #[case::negative_limit("SHOW PARTITIONS FROM TABLE books LIMIT -1")]
    #[case::text_limit("SHOW PARTITIONS FROM TABLE books LIMIT 'ten'")]
    #[case::prefix_not_string("SHOW PARTITIONS FROM TABLE books STARTS WITH acme")]
    #[case::unbound_prefix_placeholder("SHOW PARTITIONS FROM TABLE books STARTS WITH $1")]
    #[case::limit_before_prefix("SHOW PARTITIONS FROM TABLE books LIMIT 10 STARTS WITH 'acme'")]
    #[case::variable("SHOW transaction_isolation")]
    fn show_rejected(#[case] sql: &str) {
        assert!(convert(sql).is_err(), "{sql}");
    }
}
