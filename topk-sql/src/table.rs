use std::fmt::{self, Display};

use sqlparser::ast::ObjectName;
use topk_rs::{Client, CollectionClient};

use crate::{Error, sql_invalid};

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
        match name.0.as_slice() {
            [ident] => {
                let value = ident
                    .as_ident()
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
            // Two-part name is treated as schema.collection — schema is ignored.
            [_schema, collection] => {
                let value = collection
                    .as_ident()
                    .ok_or_else(|| Error::Invalid("table name must be an identifier".to_string()))?
                    .value
                    .as_str();

                if let Some((coll, part)) = value.split_once('$') {
                    Ok(Self::Partition(coll.to_string(), part.to_string()))
                } else {
                    Ok(Self::Collection(value.to_string()))
                }
            }
            _ => sql_invalid!(
                "invalid table reference; supported forms: \
                collection, schema.collection, \
                collection$partition, schema.collection$partition, \
                collection PARTITION name"
            ),
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
