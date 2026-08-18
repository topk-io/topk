use futures::TryStreamExt;
use indexmap::IndexMap;
use mongodb::bson::{doc, Bson as Wire, Document as BsonDoc};
use mongodb::{Client, Database};

use crate::import::error::Error;
use crate::import::source::codec::bson;
use crate::import::source::Record;
use crate::import::spec::{Field, Target, Type};

use super::{Records, Table};

const SAMPLE: i64 = 100;

pub struct Mongo {
    db: Database,
}

impl Mongo {
    pub async fn connect(url: &str) -> Result<Mongo, Error> {
        let client = Client::with_uri_str(url).await?;
        let db = client.default_database().ok_or_else(|| {
            Error::InvalidArgument(
                "mongodb url must include a database, e.g. mongodb://host/dbname".into(),
            )
        })?;
        Ok(Mongo { db })
    }

    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        let names = self.db.list_collection_names().await?;
        let mut tables = Vec::with_capacity(names.len());
        for from in names {
            let mut cursor = self
                .db
                .collection::<BsonDoc>(&from)
                .aggregate([doc! { "$sample": { "size": SAMPLE } }])
                .await?;

            // Type on first sight; array length agreed across samples, or 0 on
            // disagreement (0 never promotes to a vector).
            let mut fields: IndexMap<String, (Type, Option<u32>)> = IndexMap::new();
            while let Some(document) = cursor.try_next().await? {
                for (key, value) in &document {
                    let (ty, len) = fields
                        .entry(key.clone())
                        .or_insert_with(|| (bson::ty(value), None));
                    // Decimal128 in any sampled doc forces Text: f64 can't hold its precision.
                    if matches!(value, Wire::Decimal128(_)) {
                        *ty = Type::Text;
                    }
                    if let Wire::Array(items) = value {
                        let seen = items.len() as u32;
                        *len = Some(match *len {
                            None => seen,
                            Some(agreed) if agreed == seen => seen,
                            Some(_) => 0,
                        });
                    }
                }
            }
            let columns = fields
                .into_iter()
                .map(|(name, (ty, len))| {
                    let field = match len {
                        Some(dim) if dim > 0 && matches!(ty, Type::FloatList) => Field {
                            ty: Type::F32Vector,
                            dim: Some(dim),
                            ..Default::default()
                        },
                        _ => Field {
                            ty,
                            ..Default::default()
                        },
                    };
                    (name, field)
                })
                .collect();
            tables.push(Table {
                collection_hint: Some(from.clone()),
                from,
                columns,
                primary_key: None,
            });
        }
        Ok(tables)
    }

    pub async fn stream(&self, target: &Target) -> Result<Records, Error> {
        let filter = match target.filter.as_deref() {
            Some(f) => {
                let json: serde_json::Value = serde_json::from_str(f).map_err(|e| {
                    Error::InvalidArgument(format!(
                        "filter {f:?} is not JSON — mongodb filters are query documents, \
                         e.g. '{{\"year\": {{\"$gt\": 2000}}}}' ({e})"
                    ))
                })?;
                mongodb::bson::to_document(&json)
                    .map_err(|e| Error::InvalidArgument(format!("invalid mongo filter: {e}")))?
            }
            None => doc! {},
        };
        let collection = self.db.collection::<BsonDoc>(&target.from);
        let cursor = {
            // No 10m idle timeout under sink backpressure; abandoned cursors
            // are still reaped with their session (~30m).
            let mut find = collection.find(filter).no_cursor_timeout(true);
            if let Some(n) = target.limit {
                find = find.limit(n as i64);
            }
            find.await?
        };

        let stream = async_stream::stream! {
            let mut cursor = cursor;
            loop {
                match cursor.try_next().await {
                    Ok(Some(document)) => {
                        let row = document
                            .into_iter()
                            .map(|(key, value)| Ok((key, bson::value(value)?)))
                            .collect::<Result<Record, Error>>();
                        yield row.map_err(|e| Error::Row(Box::new(e)));
                    }
                    Ok(None) => break,
                    Err(e) => {
                        yield Err(e.into());
                        break;
                    }
                }
            }
        };
        Ok(Box::pin(stream))
    }
}
