use std::fmt::{self, Display};

use sqlparser::ast::ObjectName;
use topk_rs::{Client, CollectionClient};

use crate::{Error, ObjectNameExt, sql_invalid};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Table {
    Collection(String),
    Partition(String, String),
}

impl Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Table::Collection(collection) => write!(f, "{collection}"),
            Table::Partition(collection, partition) => write!(f, "{collection}${partition}"),
        }
    }
}

impl Table {
    pub fn new(name: ObjectName) -> Result<Self, Error> {
        sql_invalid!(
            name.0.len() > 2,
            "invalid table reference; supported forms: \
            collection, schema.collection, \
            collection$partition, schema.collection$partition, \
            collection PARTITION name"
        );

        // Only the `public` schema is supported; it resolves to the bare collection.
        if let Some(schema) = name.schema() {
            sql_invalid!(
                !schema.eq_ignore_ascii_case("public"),
                "unknown schema '{schema}'; only 'public' is supported"
            );
        }

        let value = name
            .0
            .last()
            .and_then(|part| part.as_ident())
            .ok_or_else(|| Error::Invalid("table name must be an identifier".to_string()))?
            .value
            .as_str();

        if let Some((collection, partition)) = value.split_once('$') {
            Ok(Self::Partition(
                collection.to_string(),
                partition.to_string(),
            ))
        } else {
            Ok(Self::Collection(value.to_string()))
        }
    }

    pub fn collection(&self) -> &str {
        match self {
            Table::Collection(collection) => collection.as_str(),
            Table::Partition(collection, _) => collection.as_str(),
        }
    }

    pub fn configure(self, client: Client) -> CollectionClient {
        match self {
            Table::Collection(collection) => client.collection(collection),
            Table::Partition(collection, partition) => {
                client.collection(collection).partition(partition)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use sqlparser::ast::{Statement as SqlStatement, TableObject};

    use super::*;
    use crate::{ObjectNameExt, parse_sql};

    fn name(table: &str) -> ObjectName {
        let sql = format!("INSERT INTO {table} (_id) VALUES ('1')");
        match parse_sql(&sql).unwrap().pop().unwrap() {
            SqlStatement::Insert(i) => match i.table {
                TableObject::TableName(name) => name,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    #[rstest]
    #[case("books", Table::Collection("books".into()))]
    #[case("public.books", Table::Collection("books".into()))]
    #[case("PUBLIC.books", Table::Collection("books".into()))]
    #[case("books$part", Table::Partition("books".into(), "part".into()))]
    #[case("public.books$part", Table::Partition("books".into(), "part".into()))]
    fn new_ok(#[case] table: &str, #[case] expected: Table) {
        assert_eq!(Table::new(name(table)).unwrap(), expected);
    }

    #[rstest]
    #[case(
        "myschema.books",
        "unknown schema 'myschema'; only 'public' is supported"
    )]
    #[case(
        "information_schema.books",
        "unknown schema 'information_schema'; only 'public' is supported"
    )]
    fn new_rejects_non_public_schema(#[case] table: &str, #[case] message: &str) {
        match Table::new(name(table)).unwrap_err() {
            Error::Invalid(msg) => assert_eq!(msg, message),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[rstest]
    #[case("books", None)]
    #[case("public.books", Some("public"))]
    #[case("pg_catalog.pg_class", Some("pg_catalog"))]
    fn schema(#[case] table: &str, #[case] expected: Option<&str>) {
        assert_eq!(name(table).schema(), expected);
    }
}
