use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};
use prost::Message;
use topk_rs::proto::v1::data::{sparse_vector, value::Value as Inner, Document, Value};
use topk_rs::{Client, CollectionClient};

use crate::import::ddl::{self, Schema};
use crate::import::error::Error;
use crate::import::source::codec::{floats, int_from_f64, int_from_str, ints};
use crate::import::source::{Record, Source};
use crate::import::spec::Target;
use crate::import::spec::{Field, Type};
use crate::import::ID;

const BATCH_BYTES: usize = 8 * 1024 * 1024;
const MAX_DOC_BYTES: usize = 128 * 1024;
const UPSERTS: usize = 4;

#[derive(Default, serde::Serialize)]
pub struct Outcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
}

pub async fn documents<'a>(
    source: &Source,
    target: &'a Target,
) -> Result<impl Stream<Item = Result<Document, Error>> + 'a, Error> {
    let rows = source.stream(target).await?;
    Ok(rows.map(move |row| document(target, row?)))
}

pub async fn load(
    client: &Client,
    name: &str,
    source: &Source,
    target: &Target,
    // The collection's schema when it does not exist yet: created at the first
    // flush, so a run that reads nothing (bad filter, --limit 0) creates nothing.
    mut fresh: Option<Schema>,
    continue_on_error: bool,
    mut progress: impl FnMut(usize),
) -> Result<Outcome, Error> {
    let mut collection = client.collection(name);
    if let Some(partition) = &target.partition {
        collection = collection.partition(partition);
    }
    let mut rows = documents(source, target).await?;
    let mut batch: Vec<Document> = Vec::new();
    let mut bytes = 0;
    let mut outcome = Outcome::default();
    let mut inflight = FuturesUnordered::new();

    loop {
        match rows.next().await {
            Some(Ok(doc)) => {
                bytes += doc.encoded_len();
                batch.push(doc);
                outcome.rows += 1;
                progress(outcome.rows);
            }
            Some(Err(e)) if continue_on_error && e.skippable() => {
                crate::output::warn(format!("{name}: skipped {e}"));
                outcome.failed += 1;
                continue;
            }
            Some(Err(e)) => return Err(e),
            None => break,
        }
        if bytes >= BATCH_BYTES {
            bytes = 0;
            while inflight.len() >= UPSERTS {
                if let Some(done) = inflight.next().await {
                    done?;
                }
            }
            if let Some(schema) = fresh.take() {
                ddl::create(client, name, schema).await?;
            }
            inflight.push(upsert(collection.clone(), std::mem::take(&mut batch)));
        }
    }
    if !batch.is_empty() {
        if let Some(schema) = fresh.take() {
            ddl::create(client, name, schema).await?;
        }
        inflight.push(upsert(collection.clone(), batch));
    }
    while let Some(done) = inflight.next().await {
        done?;
    }
    Ok(outcome)
}

async fn upsert(collection: CollectionClient, docs: Vec<Document>) -> Result<(), Error> {
    collection.upsert(docs).await?;
    Ok(())
}

pub fn document(target: &Target, record: Record) -> Result<Document, Error> {
    let id_column = target.id.as_deref().unwrap_or(ID);
    let id = match record.iter().find(|(key, _)| key == id_column) {
        Some((_, value)) => id_string(id_column, value.clone())?,
        None => {
            let seen: Vec<_> = record.iter().map(|(key, _)| key.as_str()).collect();
            return Err(Error::Id(
                id_column.to_string(),
                format!(
                    "id column not present in this row, which has: {}",
                    seen.join(", ")
                ),
            ));
        }
    };
    let fail = |field: Option<&str>, source: Error| Error::Doc {
        id: id.clone(),
        field: field.map(str::to_string),
        source: Box::new(source),
    };

    // The spec is a whitelist: columns it does not name are dropped. Several
    // fields may read one column — the id included.
    let mut pairs: Vec<(String, Value)> = Vec::with_capacity(target.fields.len() + 1);
    for (key, value) in record {
        for (name, field) in target
            .fields
            .iter()
            .filter(|(name, field)| field.from.as_deref().unwrap_or(name.as_str()) == key)
        {
            let value = coerce(value.clone(), field).map_err(|e| fail(Some(name), e))?;
            pairs.push((name.clone(), value));
        }
    }
    for (name, field) in &target.fields {
        if field.required
            && !pairs
                .iter()
                .any(|(key, value)| key == name && value.as_null().is_none())
        {
            return Err(fail(
                Some(name),
                Error::InvalidArgument("required field is missing".to_string()),
            ));
        }
    }

    pairs.push((ID.to_string(), Value::string(id.clone())));
    let doc = Document::from(pairs);
    let size = doc.encoded_len();
    if size > MAX_DOC_BYTES {
        return Err(fail(
            None,
            Error::InvalidArgument(format!(
                "document is {}, over the {} limit ({}) — set `truncate = <chars>` on its \
                 largest text field, trim it in the source, exclude it with `filter`, or \
                 re-run with --continue-on-error",
                bytesize::ByteSize(size as u64).to_string_as(true),
                bytesize::ByteSize(MAX_DOC_BYTES as u64).to_string_as(true),
                largest_fields(&doc)
            )),
        ));
    }
    Ok(doc)
}

fn largest_fields(doc: &Document) -> String {
    let mut sizes: Vec<(&String, usize)> = doc
        .fields
        .iter()
        .map(|(name, value)| (name, value.encoded_len()))
        .collect();
    sizes.sort_by_key(|(_, size)| std::cmp::Reverse(*size));
    sizes
        .iter()
        .take(3)
        .map(|(name, size)| {
            format!("{name} {}", bytesize::ByteSize(*size as u64).to_string_as(true))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn id_string(id: &str, value: Value) -> Result<String, Error> {
    if value.as_null().is_some() {
        return Err(Error::Id(id.to_string(), "id is null".to_string()));
    }
    let rendered = match serde_json::Value::try_from(value) {
        Ok(serde_json::Value::String(s)) => s,
        // Excel stores every number as a double; "1.0" and "1" must be one id,
        // or re-importing the same data from xlsx duplicates every document.
        Ok(serde_json::Value::Number(n)) if n.is_f64() => {
            let f = n.as_f64().unwrap();
            int_from_f64(f).map_or_else(|| f.to_string(), |i| i.to_string())
        }
        Ok(other) => other.to_string(),
        Err(_) => {
            return Err(Error::Id(
                id.to_string(),
                "non-finite numeric value cannot be a document id".to_string(),
            ))
        }
    };
    if rendered.is_empty() {
        return Err(Error::Id(
            id.to_string(),
            "empty value cannot be a document id".to_string(),
        ));
    }
    Ok(rendered)
}

pub fn coerce(value: Value, field: &Field) -> Result<Value, Error> {
    if value.as_null().is_some() {
        return Ok(value);
    }

    Ok(match field.ty {
        Type::Text => {
            let mut text = match value.as_binary() {
                Some(bytes) => match std::str::from_utf8(bytes) {
                    Ok(text) => text.to_string(),
                    Err(_) => return Err(Error::InvalidArgument(
                        "declared as text but the bytes are not valid UTF-8; declare `bytes` to \
                         keep them as binary"
                            .to_string(),
                    )),
                },
                None => match value.value.as_ref() {
                    Some(Inner::String(s)) => s.clone(),
                    Some(Inner::Bool(b)) => b.to_string(),
                    Some(Inner::I32(n)) => n.to_string(),
                    Some(Inner::I64(n)) => n.to_string(),
                    Some(Inner::U32(n)) => n.to_string(),
                    Some(Inner::U64(n)) => n.to_string(),
                    Some(Inner::F32(f)) => f.to_string(),
                    Some(Inner::F64(f)) => f.to_string(),
                    _ => serde_json::Value::try_from(value.clone())?.to_string(),
                },
            };
            if let Some(chars) = field.truncate {
                if let Some((at, _)) = text.char_indices().nth(chars) {
                    text.truncate(at);
                }
            }
            Value::string(text)
        }
        Type::Int => Value::i64(
            match value.value.as_ref() {
                Some(Inner::I32(n)) => Some(*n as i64),
                Some(Inner::I64(n)) => Some(*n),
                Some(Inner::U32(n)) => Some(*n as i64),
                Some(Inner::U64(n)) => i64::try_from(*n).ok(),
                Some(Inner::F32(f)) => int_from_f64(*f as f64),
                Some(Inner::F64(f)) => int_from_f64(*f),
                Some(Inner::Bool(b)) => Some(*b as i64),
                Some(Inner::String(s)) => int_from_str(s),
                _ => None,
            }
            .ok_or(Error::CannotCoerce(field.ty))?,
        ),
        Type::Float => Value::f64(
            match value.value.as_ref() {
                Some(Inner::I32(n)) => Some(*n as f64),
                Some(Inner::I64(n)) => Some(*n as f64),
                Some(Inner::U32(n)) => Some(*n as f64),
                Some(Inner::U64(n)) => Some(*n as f64),
                Some(Inner::F32(f)) => Some(*f as f64),
                Some(Inner::F64(f)) => Some(*f),
                Some(Inner::String(s)) => s.trim().parse().ok(),
                _ => None,
            }
            .ok_or(Error::CannotCoerce(field.ty))?,
        ),
        Type::Bool => Value::bool(
            match value.value.as_ref() {
                Some(Inner::Bool(b)) => Some(*b),
                Some(Inner::I32(n)) => Some(*n != 0),
                Some(Inner::I64(n)) => Some(*n != 0),
                Some(Inner::U32(n)) => Some(*n != 0),
                Some(Inner::U64(n)) => Some(*n != 0),
                Some(Inner::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
                    "true" | "t" | "1" | "yes" | "y" => Some(true),
                    "false" | "f" | "0" | "no" | "n" => Some(false),
                    _ => None,
                },
                _ => None,
            }
            .ok_or(Error::CannotCoerce(field.ty))?,
        ),
        Type::TextList => {
            let value = json_text(value)?;
            value
                .as_string_list()
                .map(|s| Value::list(s.to_vec()))
                .ok_or(Error::CannotCoerce(field.ty))?
        }
        Type::IntList => {
            let value = json_text(value)?;
            ints(&value)
                .or_else(|| {
                    floats(&value).and_then(|ns| ns.iter().map(|&n| int_from_f64(n)).collect())
                })
                .map(Value::list)
                .ok_or(Error::CannotCoerce(field.ty))?
        }
        Type::FloatList => {
            let value = json_text(value)?;
            floats(&value)
                .map(|ns| Value::list(ns.iter().map(|&n| n as f32).collect::<Vec<f32>>()))
                .ok_or(Error::CannotCoerce(field.ty))?
        }
        Type::F32Vector => {
            Value::list(elements(value, field)?.iter().map(|&n| n as f32).collect::<Vec<f32>>())
        }
        Type::F16Vector => Value::list(
            elements(value, field)?
                .iter()
                .map(|&n| half::f16::from_f32(n as f32))
                .collect::<Vec<_>>(),
        ),
        Type::F8Vector => Value::list(
            elements(value, field)?
                .iter()
                .map(|&n| float8::F8E4M3::from_f32(n as f32))
                .collect::<Vec<_>>(),
        ),
        Type::U8Vector | Type::BinaryVector => {
            Value::list(exact_ints::<u8>(&elements(value, field)?, field.ty)?)
        }
        Type::I8Vector => Value::list(exact_ints::<i8>(&elements(value, field)?, field.ty)?),
        Type::F32Matrix | Type::F16Matrix | Type::F8Matrix | Type::U8Matrix | Type::I8Matrix => {
            let cols = field
                .cols
                .ok_or_else(|| Error::InvalidArgument(format!("{} requires `cols`", field.ty)))?;
            match value.value.as_ref() {
                Some(Inner::Matrix(m)) if m.num_cols == cols => value,
                Some(Inner::Matrix(m)) => {
                    return Err(Error::InvalidArgument(format!(
                        "matrix has {} columns, declared cols={cols}",
                        m.num_cols
                    )))
                }
                _ => return Err(Error::CannotCoerce(field.ty)),
            }
        }
        Type::Struct => {
            let value = json_text(value)?;
            if value.as_struct().is_none() {
                return Err(Error::CannotCoerce(field.ty));
            }
            value
        }
        Type::F32SparseVector => {
            let (indices, values) = sparse_pairs(value, field.ty)?;
            Value::f32_sparse_vector(indices, values.iter().map(|&n| n as f32).collect())
        }
        Type::F16SparseVector => {
            let (indices, values) = sparse_pairs(value, field.ty)?;
            Value::f16_sparse_vector(
                indices,
                values
                    .iter()
                    .map(|&n| half::f16::from_f32(n as f32))
                    .collect(),
            )
        }
        Type::F8SparseVector => {
            let (indices, values) = sparse_pairs(value, field.ty)?;
            Value::f8_sparse_vector(
                indices,
                values
                    .iter()
                    .map(|&n| float8::F8E4M3::from_f32(n as f32))
                    .collect(),
            )
        }
        Type::U8SparseVector => {
            let (indices, values) = sparse_pairs(value, field.ty)?;
            Value::u8_sparse_vector(indices, exact_ints::<u8>(&values, field.ty)?)
        }
        Type::I8SparseVector => {
            let (indices, values) = sparse_pairs(value, field.ty)?;
            Value::i8_sparse_vector(indices, exact_ints::<i8>(&values, field.ty)?)
        }
        Type::Bytes if value.as_binary().is_some() => value,
        Type::Bytes => return Err(Error::CannotCoerce(field.ty)),
    })
}

/// Container types accept JSON in a string cell (CSV, TEXT columns).
fn json_text(value: Value) -> Result<Value, Error> {
    match value.as_string() {
        Some(text) => Ok(serde_json::from_str::<topk_rs::json::Value>(text)?.into_inner()),
        None => Ok(value),
    }
}

/// Numeric elements of a declared vector, checked against `dim`. Accepts a
/// JSON string cell — pgvector's text form (`"[1,2,3]"`) arrives that way.
fn elements(value: Value, field: &Field) -> Result<Vec<f64>, Error> {
    let dim = field
        .dim
        .ok_or_else(|| Error::InvalidArgument(format!("{} requires `dim`", field.ty)))?;
    let value = json_text(value)?;
    let nums = floats(&value).ok_or(Error::CannotCoerce(field.ty))?;
    if nums.len() != dim as usize {
        return Err(Error::InvalidArgument(format!(
            "vector has {} values, declared dim={dim}",
            nums.len()
        )));
    }
    Ok(nums)
}

/// (indices, values) sorted by index. topk's json layer already folds
/// numeric-key objects into an f32 sparse vector; sources hand us the struct form.
fn sparse_pairs(value: Value, ty: Type) -> Result<(Vec<u32>, Vec<f64>), Error> {
    let value = json_text(value)?;
    let mut pairs: Vec<(u32, f64)> = match value.value.as_ref() {
        Some(Inner::SparseVector(sparse)) => match sparse.values.as_ref() {
            Some(sparse_vector::Values::F32(f)) => sparse
                .indices
                .iter()
                .copied()
                .zip(f.values.iter().map(|&v| v as f64))
                .collect(),
            _ => return Err(Error::CannotCoerce(ty)),
        },
        _ => {
            let entries = value.as_struct().ok_or(Error::CannotCoerce(ty))?;
            let mut pairs = Vec::with_capacity(entries.len());
            for (key, entry) in entries {
                let index: u32 = key.trim().parse().map_err(|_| Error::CannotCoerce(ty))?;
                let number = match entry.value.as_ref() {
                    Some(Inner::I32(n)) => *n as f64,
                    Some(Inner::I64(n)) => *n as f64,
                    Some(Inner::U32(n)) => *n as f64,
                    Some(Inner::U64(n)) => *n as f64,
                    Some(Inner::F32(f)) => *f as f64,
                    Some(Inner::F64(f)) => *f,
                    _ => return Err(Error::CannotCoerce(ty)),
                };
                pairs.push((index, number));
            }
            pairs
        }
    };
    pairs.sort_by_key(|(index, _)| *index);
    Ok(pairs.into_iter().unzip())
}

/// Integer elements must be exact: no fractions, no out-of-range wrapping.
fn exact_ints<T: TryFrom<i64>>(nums: &[f64], ty: Type) -> Result<Vec<T>, Error> {
    nums.iter()
        .map(|&n| {
            int_from_f64(n)
                .and_then(|n| T::try_from(n).ok())
                .ok_or(Error::CannotCoerce(ty))
        })
        .collect()
}
