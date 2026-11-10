use std::str::FromStr;

use futures::TryStreamExt;
use indexmap::IndexMap;
use mongodb::bson::{doc, Bson as Wire, Document as BsonDoc};
use mongodb::{Client, Database};
use topk_rs::proto::v1::control::{field_type_list::ListValueType, FieldSpec};
use topk_rs::proto::v1::data::Value;
use url::Url;

use crate::import::error::Error;
use crate::import::source::Record;
use crate::import::spec::Target;
use crate::import::ID;

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
            let mut fields: IndexMap<String, (FieldSpec, Option<u32>)> = IndexMap::new();
            while let Some(document) = cursor.try_next().await? {
                for (key, value) in &document {
                    if matches!(value, Wire::Null | Wire::Undefined) {
                        continue;
                    }
                    let (kind, len) = fields
                        .entry(key.clone())
                        .or_insert_with(|| (ty(value), None));
                    // Decimal128 in any sampled doc forces Text: f64 can't hold its precision.
                    if matches!(value, Wire::Decimal128(_)) {
                        *kind = FieldSpec::text(false);
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
                .map(|(name, (kind, len))| {
                    let field = match len.filter(|dim| *dim > 0) {
                        Some(dim) if kind == FieldSpec::list(false, ListValueType::Float) => {
                            FieldSpec::f32_vector(dim, false)
                        }
                        _ => kind,
                    };
                    (name, Some(field))
                })
                .collect();
            tables.push(Table {
                collection_hint: Some(from.clone()),
                from,
                columns,
                key: Some(ID.to_string()),
            });
        }
        Ok(tables)
    }

    /// Ordered by the id (`_id` is always indexed); the cursor is the last id as
    /// canonical extended JSON, which keeps its BSON type.
    pub fn chunks(
        &self,
        target: &Target,
        after: Option<super::Cursor>,
    ) -> Result<ChunkStream, Error> {
        let Filter(mut filter) = target
            .filter
            .as_deref()
            .map(str::parse)
            .transpose()?
            .unwrap_or_default();
        if let Some(cursor) = after {
            let key = match cursor {
                super::Cursor::Mongo(key) => key,
                super::Cursor::Key(key) if target.id == ID => key,
                super::Cursor::Key(_) => {
                    return Err(Error::InvalidArgument(
                        "legacy Mongo checkpoint uses a custom id column; start a new import"
                            .into(),
                    ))
                }
                other => {
                    return Err(Error::InvalidArgument(format!(
                        "resume cursor {other} is not a Mongo key"
                    )))
                }
            };
            let value = Wire::try_from(serde_json::from_str::<serde_json::Value>(&key)?).map_err(
                |error| Error::InvalidArgument(format!("bad resume cursor {key:?}: {error}")),
            )?;
            filter =
                doc! { "$and": [filter, { "$expr": { "$gt": ["$_id", { "$literal": value }] } }] };
        }
        let collection = self.db.collection::<BsonDoc>(&target.from);
        let limit = target.limit;

        let stream = async_stream::stream! {
            // No 10m idle timeout under sink backpressure.
            let mut find = collection
                .find(filter)
                .sort(doc! { ID: 1 })
                .no_cursor_timeout(true);
            // Mongo reads `limit(0)` as no limit; a resume at its limit has nothing left.
            match limit {
                Some(0) => return,
                Some(n) => find = find.limit(n as i64),
                None => {}
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
                            .get(ID)
                            .map(|value| super::Cursor::Mongo(value.clone().into_canonical_extjson().to_string()));
                        let row = document
                            .into_iter()
                            .map(|(key, value)| Ok((key, self::value(value)?)))
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
        Ok(Box::pin(stream))
    }
}

fn ty(input: &Wire) -> FieldSpec {
    match input {
        Wire::Boolean(_) => FieldSpec::boolean(false),
        Wire::Int32(_) | Wire::Int64(_) => FieldSpec::integer(false),
        Wire::Double(_) => FieldSpec::float(false),
        Wire::Decimal128(_) => FieldSpec::text(false),
        Wire::Binary(_) => FieldSpec::bytes(false),
        Wire::Document(_) => FieldSpec::r#struct(false, Vec::<(String, FieldSpec)>::new()),
        Wire::Array(items) if items.is_empty() => FieldSpec::list(false, ListValueType::Float),
        Wire::Array(items) => {
            if items
                .iter()
                .all(|b| matches!(b, Wire::Int32(_) | Wire::Int64(_)))
            {
                FieldSpec::list(false, ListValueType::Integer)
            } else if items.iter().all(|b| {
                matches!(
                    b,
                    Wire::Int32(_) | Wire::Int64(_) | Wire::Double(_) | Wire::Decimal128(_)
                )
            }) {
                FieldSpec::list(false, ListValueType::Float)
            } else if items.iter().all(|b| matches!(b, Wire::String(_))) {
                FieldSpec::list(false, ListValueType::String)
            } else {
                FieldSpec::text(false)
            }
        }
        _ => FieldSpec::text(false),
    }
}

fn value(input: Wire) -> Result<Value, Error> {
    Ok(match input {
        Wire::Null | Wire::Undefined => Value::null(),
        Wire::Boolean(b) => Value::bool(b),
        Wire::Int32(i) => Value::i64(i as i64),
        Wire::Int64(i) => Value::i64(i),
        Wire::Double(f) => Value::f64(f),
        Wire::String(s) => Value::string(s),
        Wire::ObjectId(oid) => Value::string(oid.to_hex()),
        Wire::Binary(bin) => Value::binary(bin.bytes),
        Wire::DateTime(dt) => Value::string(
            dt.try_to_rfc3339_string()
                .unwrap_or_else(|_| dt.to_string()),
        ),
        Wire::Decimal128(d) => Value::string(d.to_string()),
        Wire::Document(document) => {
            let mut out = Vec::with_capacity(document.len());
            for (key, v) in document {
                out.push((key, value(v)?));
            }
            Value::r#struct(out)
        }
        // Empty lists default to float, same as the Arrow path.
        Wire::Array(items) if items.is_empty() => Value::list(Vec::<f32>::new()),
        Wire::Array(items) => {
            if let Some(values) = items
                .iter()
                .map(|b| match b {
                    Wire::Int32(v) => Some(i64::from(*v)),
                    Wire::Int64(v) => Some(*v),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            {
                return Ok(Value::list(values));
            }
            if let Some(values) = items
                .iter()
                .map(|b| match b {
                    Wire::Int32(v) => Some(f64::from(*v)),
                    Wire::Int64(v) => Some(*v as f64),
                    Wire::Double(v) => Some(*v),
                    Wire::Decimal128(d) => d.to_string().parse().ok(),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            {
                return Ok(Value::list(values));
            }
            if let Some(values) = items
                .into_iter()
                .map(|b| match b {
                    Wire::String(s) => Some(s),
                    _ => None,
                })
                .collect::<Option<Vec<String>>>()
            {
                return Ok(Value::list(values));
            }
            return Err(Error::InvalidArgument(
                "list mixes types; TopK lists hold numbers or strings, not both".to_string(),
            ));
        }
        other => Value::string(other.to_string()),
    })
}
