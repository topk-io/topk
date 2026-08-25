use std::collections::{BTreeSet, VecDeque};

use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};
use prost::Message;
use topk_rs::proto::v1::data::{sparse_vector, value::Value as Inner, Document, Value};
use topk_rs::{Client, CollectionClient};

use crate::import::ddl::{self, Schema};
use crate::import::error::Error;
use crate::import::source::codec::{finite, floats, id_string, int_from_f64, int_from_str, ints};
use crate::import::source::{Record, Source};
use crate::import::spec::Target;
use crate::import::spec::{Field, Type};
use crate::import::ID;

/// The documented per-document limit (docs/limits.mdx: 200KB), decimal on
/// purpose: a little strict yields our error, which names the largest fields.
const MAX_DOC_BYTES: usize = 200_000;
/// `SlowDown` never fails a run: shard capacity moves with co-tenants and
/// compaction, so no round count means "broken". Warned once a minute.
const SLOWDOWN_BACKOFF_MS: u64 = 250;
const SLOWDOWN_MAX_BACKOFF_MS: u64 = 8_000;
const SLOWDOWN_WARN_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Default, serde::Serialize)]
pub struct Outcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
    /// Encoded document bytes upserted.
    pub bytes: usize,
    pub elapsed_ms: u64,
    /// Per-upsert latency; the batch size makes the mean meaningless.
    pub upsert_p50_ms: u64,
    pub upsert_p99_ms: u64,
}

pub async fn documents<'a>(
    source: &Source,
    target: &'a Target,
    after: Option<&str>,
) -> Result<impl Stream<Item = Result<Document, Error>> + 'a, Error> {
    let chunks = source.stream(target, after).await?;
    Ok(chunks
        .flat_map(|chunk| futures::stream::iter(chunk.rows))
        .map(move |row| document(target, row?)))
}

pub async fn load(
    client: &Client,
    name: &str,
    source: &Source,
    target: &Target,
    // Schema of a collection that does not exist yet; created at the first
    // flush, so a run that reads nothing creates nothing.
    mut fresh: Option<Schema>,
    after: Option<&str>,
    continue_on_error: bool,
    batch_bytes: usize,
    // Inflight upserts for this collection: the run's budget divided by the
    // objects sharing it.
    upserts: usize,
    mut progress: impl FnMut(usize),
    // Called with a source mark once every row up to it has been upserted.
    mut checkpoint: impl FnMut(&str),
) -> Result<Outcome, Error> {
    let mut collection = client.collection(name);
    if let Some(partition) = &target.partition {
        collection = collection.partition(partition);
    }
    let mut chunks = source.stream(target, after).await?;
    let mut batch: Vec<Document> = Vec::new();
    let mut bytes = 0;
    let mut outcome = Outcome::default();
    let mut inflight = FuturesUnordered::new();
    let mut latencies: Vec<u64> = Vec::new();
    let started = std::time::Instant::now();
    // Upserts land out of order; a mark commits once every batch up to the one
    // carrying it has landed.
    let mut seq: u64 = 0;
    let mut commits = Commits::default();
    // Rows arrive before the mark that covers them, so it rides with the next
    // batch flushed.
    let mut pending: Option<String> = None;

    'chunks: while let Some(chunk) = chunks.next().await {
        for row in chunk.rows {
            let doc = match row.and_then(|record| document(target, record)) {
                Ok(doc) => doc,
                Err(Error::Expired) => {
                    eprintln!("{name}: resume cursor expired, restarting from the beginning");
                    chunks = source.stream(target, None).await?;
                    continue 'chunks;
                }
                Err(e) if continue_on_error && e.skippable() => {
                    eprintln!("{name}: skipped {e}");
                    outcome.failed += 1;
                    continue;
                }
                Err(e) => {
                    // Let inflight batches land so their marks are checkpointed.
                    while let Some(done) = inflight.next().await {
                        if let Ok((landed, _)) = done {
                            commits.land(landed, &mut checkpoint);
                        }
                    }
                    return Err(e);
                }
            };
            let len = doc.encoded_len();
            bytes += len;
            outcome.bytes += len;
            batch.push(doc);
            outcome.rows += 1;
            progress(outcome.rows);
            if bytes >= batch_bytes {
                bytes = 0;
                while inflight.len() >= upserts {
                    if let Some(done) = inflight.next().await {
                        let (landed, latency) = done?;
                        latencies.push(latency);
                        commits.land(landed, &mut checkpoint);
                    }
                }
                if let Some(schema) = fresh.take() {
                    ddl::create(client, name, schema).await?;
                }
                if let Some(mark) = pending.take() {
                    commits.marks.push_back((seq, mark));
                }
                inflight.push(upsert(seq, collection.clone(), std::mem::take(&mut batch)));
                seq += 1;
            }
        }
        if let Some(mark) = chunk.mark {
            pending = Some(mark);
        }
    }
    if !batch.is_empty() {
        if let Some(schema) = fresh.take() {
            ddl::create(client, name, schema).await?;
        }
        if let Some(mark) = pending.take() {
            commits.marks.push_back((seq, mark));
        }
        inflight.push(upsert(seq, collection.clone(), batch));
    }
    while let Some(done) = inflight.next().await {
        let (landed, latency) = done?;
        latencies.push(latency);
        commits.land(landed, &mut checkpoint);
    }
    outcome.elapsed_ms = started.elapsed().as_millis() as u64;
    latencies.sort_unstable();
    outcome.upsert_p50_ms = percentile(&latencies, 50);
    outcome.upsert_p99_ms = percentile(&latencies, 99);
    Ok(outcome)
}

#[derive(Default)]
struct Commits {
    /// Every batch below this has landed.
    low: u64,
    landed: BTreeSet<u64>,
    marks: VecDeque<(u64, String)>,
}

impl Commits {
    fn land(&mut self, seq: u64, checkpoint: &mut impl FnMut(&str)) {
        self.landed.insert(seq);
        while self.landed.remove(&self.low) {
            self.low += 1;
        }
        while self.marks.front().is_some_and(|(seq, _)| *seq < self.low) {
            let (_, mark) = self.marks.pop_front().unwrap();
            checkpoint(&mark);
        }
    }
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    match sorted.is_empty() {
        true => 0,
        false => sorted[(sorted.len() - 1) * p / 100],
    }
}

/// Upserts a batch, backing off for as long as the shard says to slow down; a
/// sleeping batch keeps its inflight slot, which sheds load by itself.
/// Returns the batch number and the latency, retries included.
async fn upsert(
    seq: u64,
    collection: CollectionClient,
    docs: Vec<Document>,
) -> Result<(u64, u64), Error> {
    let started = std::time::Instant::now();
    let mut backoff = SLOWDOWN_BACKOFF_MS;
    let mut warned = std::time::Instant::now();
    loop {
        // Cloned so a throttled batch can be replayed.
        match collection.upsert(docs.clone()).await {
            Ok(_) => return Ok((seq, started.elapsed().as_millis() as u64)),
            Err(topk_rs::Error::SlowDown(_)) => {
                if warned.elapsed() >= SLOWDOWN_WARN_EVERY {
                    warned = std::time::Instant::now();
                    eprintln!(
                        "shard capacity exceeded — throttled for {}s so far; lower -c to \
                         stop hammering it, or Ctrl-C and --resume later",
                        started.elapsed().as_secs()
                    );
                }
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                backoff = (backoff * 2).min(SLOWDOWN_MAX_BACKOFF_MS);
            }
            Err(e) => return Err(e.into()),
        }
    }
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

    // The spec is a whitelist; several fields may read one column, the id included.
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
        let (largest, binary) = largest_fields(&doc);
        return Err(fail(
            None,
            Error::InvalidArgument(format!(
                "document is {}, over the {} limit ({}) — {}, exclude it with `filter`, or \
                 re-run with --continue-on-error",
                bytesize::ByteSize(size as u64).to_string_as(true),
                bytesize::ByteSize(MAX_DOC_BYTES as u64).to_string_as(true),
                largest,
                match binary {
                    // `truncate` is text-only; half an image is not a smaller image.
                    true =>
                        "a binary field cannot be truncated: index its embedding and keep \
                             the payload behind a reference, or shrink it at the source",
                    false =>
                        "set `truncate = <chars>` on its largest text field, or trim it \
                              in the source",
                }
            )),
        ));
    }
    Ok(doc)
}

/// The three biggest fields, and whether the biggest is binary.
fn largest_fields(doc: &Document) -> (String, bool) {
    let mut sizes: Vec<(&String, &Value)> = doc.fields.iter().collect();
    sizes.sort_by_key(|(_, value)| std::cmp::Reverse(value.encoded_len()));
    let names = sizes
        .iter()
        .take(3)
        .map(|(name, value)| {
            format!(
                "{name} {}",
                bytesize::ByteSize(value.encoded_len() as u64).to_string_as(true)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let binary = sizes
        .first()
        .is_some_and(|(_, value)| value.as_binary().is_some());
    (names, binary)
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
        Type::Int => Value::i64(int_from_value(&value).ok_or(Error::CannotCoerce(field.ty))?),
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
        Type::F32Vector => Value::list(
            elements(value, field)?
                .iter()
                .map(|&n| n as f32)
                .collect::<Vec<f32>>(),
        ),
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
                _ => reshape(value, cols, field.ty)?,
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
        // Declared in the schema, carried as an epoch integer. Sources that
        // render a date as text (elasticsearch `_source`, a csv column) parse
        // to the same instant; anything else is a `text` field, not a timestamp.
        Type::Timestamp => Value::i64(
            match value.value.as_ref() {
                Some(Inner::String(s)) => epoch_millis(s),
                _ => None,
            }
            .or_else(|| int_from_value(&value))
            .ok_or(Error::CannotCoerce(field.ty))?,
        ),
    })
}

/// An exact i64 from whatever numeric shape a source produced.
fn int_from_value(value: &Value) -> Option<i64> {
    match value.value.as_ref()? {
        Inner::I32(n) => Some(*n as i64),
        Inner::I64(n) => Some(*n),
        Inner::U32(n) => Some(*n as i64),
        Inner::U64(n) => i64::try_from(*n).ok(),
        Inner::F32(f) => int_from_f64(*f as f64),
        Inner::F64(f) => int_from_f64(*f),
        Inner::Bool(b) => Some(*b as i64),
        Inner::String(s) => int_from_str(s),
        _ => None,
    }
}

/// Milliseconds since the epoch from a rendered date: RFC 3339 first, then a
/// bare date, which is midnight UTC.
fn epoch_millis(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Ok(stamp) = chrono::DateTime::parse_from_rfc3339(text) {
        return Some(stamp.timestamp_millis());
    }
    let date = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()?;
    Some(
        date.and_time(chrono::NaiveTime::MIN)
            .and_utc()
            .timestamp_millis(),
    )
}

/// Container types accept JSON in a string cell (CSV, TEXT columns).
fn json_text(value: Value) -> Result<Value, Error> {
    match value.as_string() {
        Some(text) => Ok(serde_json::from_str::<topk_rs::json::Value>(text)?.into_inner()),
        None => Ok(value),
    }
}

/// Numeric elements of a declared vector, checked against `dim`. Accepts a JSON
/// string cell (pgvector's text form) and a binary cell holding a packed array.
fn elements(value: Value, field: &Field) -> Result<Vec<f64>, Error> {
    let dim = field
        .dim
        .ok_or_else(|| Error::InvalidArgument(format!("{} requires `dim`", field.ty)))?;
    if let Some(bytes) = value.as_binary() {
        return unpack(bytes, dim as usize, field.ty);
    }
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

/// Elements of a binary cell holding a packed little-endian array. The element
/// width is `len / dim`, not the declared type's: blobs in the wild routinely
/// disagree with the target (numpy writes f64, our own datasets f16).
fn unpack(bytes: &[u8], dim: usize, ty: Type) -> Result<Vec<f64>, Error> {
    if dim == 0 || bytes.len() % dim != 0 {
        return Err(Error::InvalidArgument(format!(
            "{} bytes does not divide into dim={dim} (a binary cell decodes as a packed \
             little-endian array{})",
            bytes.len(),
            candidate_dims(bytes.len()),
        )));
    }
    let width = bytes.len() / dim;
    // Byte vectors read one byte per element; a wider cell would be lossy.
    if matches!(ty, Type::U8Vector | Type::I8Vector | Type::BinaryVector) {
        if width != 1 {
            return Err(Error::InvalidArgument(format!(
                "{} bytes over dim={dim} is {width} bytes per element, but {ty} reads 1",
                bytes.len()
            )));
        }
        return Ok(match ty {
            Type::I8Vector => bytes.iter().map(|&b| b as i8 as f64).collect(),
            _ => bytes.iter().map(|&b| f64::from(b)).collect(),
        });
    }
    // `chunks_exact` guarantees each chunk's length, so `try_into` cannot fail.
    match width {
        2 => bytes
            .chunks_exact(2)
            .map(|c| finite(f64::from(half::f16::from_le_bytes(c.try_into().unwrap()))))
            .collect(),
        4 => bytes
            .chunks_exact(4)
            .map(|c| finite(f64::from(f32::from_le_bytes(c.try_into().unwrap()))))
            .collect(),
        8 => bytes
            .chunks_exact(8)
            .map(|c| finite(f64::from_le_bytes(c.try_into().unwrap())))
            .collect(),
        _ => Err(Error::InvalidArgument(format!(
            "{} bytes over dim={dim} is {width} bytes per element; a packed binary cell \
             holds 2 (f16), 4 (f32) or 8 (f64) bytes per element",
            bytes.len()
        ))),
    }
}

/// A flat numeric list as a matrix `cols` wide; multi-vector sources flatten
/// their rows (colbert: one `FLOAT[]` per document). Not for binary: without a
/// `dim` the element width is ambiguous (16 KiB over cols=128 is 32 f32 rows
/// or 64 f16 rows).
fn reshape(value: Value, cols: u32, ty: Type) -> Result<Value, Error> {
    let nums = floats(&json_text(value)?).ok_or(Error::CannotCoerce(ty))?;
    if cols == 0 || nums.len() % cols as usize != 0 {
        return Err(Error::InvalidArgument(format!(
            "{} values do not divide into cols={cols} (a flat list becomes a matrix \
             cols wide; rows follow from the length)",
            nums.len()
        )));
    }
    Ok(match ty {
        Type::F32Matrix => {
            Value::matrix(cols, nums.iter().map(|&n| n as f32).collect::<Vec<f32>>())
        }
        Type::F16Matrix => Value::matrix(
            cols,
            nums.iter()
                .map(|&n| half::f16::from_f32(n as f32))
                .collect::<Vec<_>>(),
        ),
        Type::F8Matrix => Value::matrix(
            cols,
            nums.iter()
                .map(|&n| float8::F8E4M3::from_f32(n as f32))
                .collect::<Vec<_>>(),
        ),
        Type::U8Matrix => Value::matrix(cols, exact_ints::<u8>(&nums, ty)?),
        Type::I8Matrix => Value::matrix(cols, exact_ints::<i8>(&nums, ty)?),
        _ => return Err(Error::CannotCoerce(ty)),
    })
}

/// The dims a byte length could mean, for the error that says `dim` is wrong.
fn candidate_dims(len: usize) -> String {
    let dims: Vec<String> = [(2, "f16"), (4, "f32"), (8, "f64")]
        .iter()
        .filter(|(width, _)| len % width == 0)
        .map(|(width, name)| format!("{} as {name}", len / width))
        .collect();
    match dims.is_empty() {
        true => String::new(),
        false => format!("; this length is dim {}", dims.join(", ")),
    }
}

/// (indices, values) sorted by index, from topk's sparse form or a struct of
/// numeric keys.
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
                // duckdb unifies jsonl schemas by filling absent keys with null.
                if entry.as_null().is_some() {
                    continue;
                }
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
