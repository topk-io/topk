use std::fmt::{self, Display};

use sqlparser::ast::ObjectName;
use topk_rs::proto::v1::control::FieldIndex;
use topk_rs::{Client, CollectionClient};

use crate::{Error, sql_invalid};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Table {
    Collection(String),
    Partition(String, Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Index {
    pub table: Table,
    pub field: String,
    pub index: FieldIndex,
}

impl Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Table::Collection(collection) => write!(f, "{collection}"),
            Table::Partition(collection, Some(partition)) => {
                write!(f, "{collection}.{partition}")
            }
            Table::Partition(collection, None) => write!(f, "{collection}"),
        }
    }
}

impl Table {
    pub fn new(name: ObjectName) -> Result<Self, Error> {
        match name.0.as_slice() {
            [collection] => Ok(Self::Collection(collection.value.clone())),
            [collection, partition] => Ok(Self::Partition(
                collection.value.clone(),
                Some(partition.value.clone()),
            )),
            _ => sql_invalid!("table name must be <collection> or <collection>.<partition>"),
        }
    }

    pub fn collection(&self) -> &str {
        match self {
            Table::Collection(collection) => collection.as_str(),
            Table::Partition(collection, _) => collection.as_str(),
        }
    }

    /// Configure `topk_rs::Client` for this table
    pub fn configure(self, client: Client) -> CollectionClient {
        match self {
            Table::Collection(collection) | Table::Partition(collection, None) => {
                client.collection(collection)
            }
            Table::Partition(collection, Some(partition)) => {
                client.collection(collection).partition(partition)
            }
        }
    }
}
