use std::str::FromStr;

use futures::TryStreamExt;
use indexmap::IndexMap;
use mongodb::bson::{doc, Bson as Wire, Document as BsonDoc};
use mongodb::{Client, Database};
use url::Url;

use crate::import::error::Error;
use crate::import::source::codec::bson;
use crate::import::source::Record;
use crate::import::spec::{Element, Field, Target, Type};

use super::{Chunk, ChunkStream, Table};

#[derive(Clone)]
pub struct Mongo {
    db: Database,
}

/// A query document; the default matches everything.
#[derive(Default)]
pub struct Filter(BsonDoc);

impl FromStr for Filter {
    type Err = Error;

    fn from_str(filter: &str) -> Result<Filter, Error> {
        serde_json::from_str::<serde_json::Value>(filter)
            .map_err(|e| e.to_string())
            .and_then(|json| mongodb::bson::to_document(&json).map_err(|e| e.to_string()))
            .map(Filter)
            .map_err(|e| {
                Error::InvalidArgument(format!(
                    "filter {filter:?} is not a mongodb query document, \
                     e.g. '{{\"year\": {{\"$gt\": 2000}}}}' ({e})"
                ))
            })
    }
}

/// The last id read, kept as BSON so `$gt` compares with its original type.
pub struct Cursor(Wire);

impl TryFrom<super::Cursor> for Cursor {
    type Error = Error;

    fn try_from(cursor: super::Cursor) -> Result<Cursor, Error> {
        let key = cursor.key()?;
        let json: serde_json::Value = serde_json::from_str(&key)?;
        Wire::try_from(json)
            .map(Cursor)
            .map_err(|e| Error::InvalidArgument(format!("bad resume cursor {key:?}: {e}")))
    }
}

impl Mongo {
    pub async fn connect(url: &Url) -> Result<Mongo, Error> {
        let client = Client::with_uri_str(url.as_str()).await?;
        let db = client
            .default_database()
            .ok_or_else(|| Error::InvalidArgument(format!("{url} names no database")))?;
        Ok(Mongo { db })
    }

    pub async fn catalog(&self) -> Result<Vec<Table>, Error> {
        let names = self.db.list_collection_names().await?;
        let mut tables = Vec::with_capacity(names.len());
        for from in names {
            let mut cursor = self
                .db
                .collection::<BsonDoc>(&from)
                .aggregate([doc! { "$sample": { "size": 1000 } }])
                .await?;

            // Type on first sight; array length agreed across samples, or 0 on
            // disagreement (0 never promotes to a vector).
            let mut fields: IndexMap<String, (Type, Option<u32>)> = IndexMap::new();
            while let Some(document) = cursor.try_next().await? {
                for (key, value) in &document {
                    if matches!(value, Wire::Null | Wire::Undefined) {
                        continue;
                    }
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
                            ty: Type::Vector(Element::F32),
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

    /// Ordered by the id (`_id` is always indexed); the cursor is the last id as
    /// canonical extended JSON, which keeps its BSON type.
    pub fn chunks(
        &self,
        target: &Target,
        filter: Option<Filter>,
        after: Option<Cursor>,
    ) -> ChunkStream {
        let id = target.id_column().to_string();
        let Filter(mut filter) = filter.unwrap_or_default();
        if let Some(Cursor(value)) = after {
            filter = doc! { "$and": [filter, { &id: { "$gt": value } }] };
        }
        let collection = self.db.collection::<BsonDoc>(&target.from);
        let limit = target.limit;

        let stream = async_stream::stream! {
            // No 10m idle timeout under sink backpressure; disk for the sort,
            // which is unindexed when `id` names a custom column.
            let mut find = collection
                .find(filter)
                .sort(doc! { &id: 1 })
                .allow_disk_use(true)
                .no_cursor_timeout(true);
            if let Some(n) = limit {
                find = find.limit(n as i64);
            }
            let mut cursor = match find.await {
                Ok(cursor) => cursor,
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            };
            let mut rows: Vec<Result<Record, Error>> = Vec::new();
            let mut mark = None;
            loop {
                match cursor.try_next().await {
                    Ok(Some(document)) => {
                        mark = document
                            .get(&id)
                            .map(|value| super::Cursor::Key(value.clone().into_canonical_extjson().to_string()));
                        let row = document
                            .into_iter()
                            .map(|(key, value)| Ok((key, bson::value(value)?)))
                            .collect::<Result<Record, Error>>();
                        rows.push(row);
                        if rows.len() == 1000 {
                            yield Ok(Chunk { rows: std::mem::take(&mut rows), cursor: mark.take() });
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        if !rows.is_empty() {
                            yield Ok(Chunk { rows: std::mem::take(&mut rows), cursor: mark.take() });
                        }
                        yield Err(e.into());
                        return;
                    }
                }
            }
            if !rows.is_empty() {
                yield Ok(Chunk { rows, cursor: mark });
            }
        };
        Box::pin(stream)
    }
}
