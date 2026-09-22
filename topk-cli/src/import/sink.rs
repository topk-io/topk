use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::{stream, Stream, StreamExt, TryStreamExt};
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use prost::Message;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use topk_rs::proto::v1::data::{Document, Value};
use topk_rs::{Client, CollectionClient};

use crate::import::decode::{self, id_string};
use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::{Cursor, Record, Scan, Source};
use crate::import::spec::Target;
use crate::import::state::{Mark, State};
use crate::import::ID;

/// Per-partition buffers, so a wide fan-out gets smaller batches, not more memory.
const BUFFERED_BATCHES: usize = 8;

#[derive(Default, serde::Serialize)]
pub struct LoadOutcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
    #[serde(skip)]
    pub elapsed: Duration,
}

/// The document a target asks for, built from one source row.
pub fn build_document(target: &Target, record: Record) -> Result<Document, Error> {
    let id_column = target.id_column();
    let id = match record.iter().find(|(key, _)| key == id_column) {
        Some((_, value)) => id_string(id_column, value.clone())?,
        None => {
            let seen: Vec<_> = record.iter().map(|(key, _)| key.as_str()).collect();
            return Err(Error::Doc {
                id: None,
                field: Some(id_column.to_string()),
                source: Box::new(Error::InvalidArgument(format!(
                    "id column not present in this row, which has: {}",
                    seen.join(", ")
                ))),
            });
        }
    };
    let fail = |field: Option<&str>, source: Error| Error::Doc {
        id: Some(id.clone()),
        field: field.map(str::to_string),
        source: Box::new(source),
    };

    // The spec is a whitelist; several fields may read one column, the id included.
    let mut pairs: Vec<(String, Value)> = Vec::with_capacity(target.fields.len() + 1);
    for (name, field) in &target.fields {
        let missing = || {
            fail(
                Some(name),
                Error::InvalidArgument("required field is missing".to_string()),
            )
        };
        let Some((_, value)) = record.iter().find(|(key, _)| key == field.source(name)) else {
            if field.required {
                return Err(missing());
            }
            continue;
        };
        let value = field
            .coerce(value.clone())
            .map_err(|e| fail(Some(name), e))?;
        if field.required && value.as_null().is_some() {
            return Err(missing());
        }
        pairs.push((name.clone(), value));
    }

    pairs.push((ID.to_string(), Value::string(id.clone())));
    let doc = Document::from(pairs);
    let size = doc.encoded_len();
    if size > MAX_DOC_BYTES {
        return Err(fail(None, Error::Oversized(size)));
    }
    Ok(doc)
}

/// This row's partition, from the value of the `partition` column, and its document.
fn build_row(target: &Target, record: Record) -> Result<(Option<String>, Document), Error> {
    let column = match target.partition.as_deref() {
        Some(column) => column,
        None => return Ok((None, build_document(target, record)?)),
    };
    let fail = |source| Error::Doc {
        id: None,
        field: Some(column.to_string()),
        source: Box::new(source),
    };
    let value = record
        .iter()
        .find(|(key, _)| key == column)
        .map(|(_, value)| value)
        .filter(|value| value.as_null().is_none())
        .ok_or_else(|| {
            fail(Error::InvalidArgument(
                "partition column is missing or null".to_string(),
            ))
        })?;
    let partition = decode::text(value.clone()).map_err(fail)?;
    let mut chars = partition.chars();
    if partition.len() > 128
        || !matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        return Err(fail(Error::InvalidArgument(format!(
            "partition value {partition:?} must start with a letter or digit, \
             contain only letters, digits, `_` or `-`, and be at most 128 bytes"
        ))));
    }
    Ok((Some(partition), build_document(target, record)?))
}

pub fn documents(
    source: &Source,
    target: &Target,
) -> Result<impl Stream<Item = Result<(Option<String>, Document), Error>>, Error> {
    let Scan { target, chunks } = source.scan(target, None)?;
    Ok(chunks
        .flat_map(|chunk| {
            stream::iter(match chunk {
                Ok(chunk) => chunk.rows,
                Err(e) => vec![Err(e)],
            })
        })
        .map(move |row| build_row(&target, row?)))
}

/// Batches in flush order, each with the source cursor it completes.
type InflightBatches = VecDeque<(JoinHandle<Result<(), Error>>, Option<Cursor>)>;

/// Collections load concurrently, as many as the source can serve at once;
/// their upserts share one budget of `-c` in flight across the run.
pub struct Sink<'a> {
    pub client: &'a Client,
    pub progress: &'a MultiProgress,
    pub budget: Arc<Semaphore>,
    pub batch_bytes: usize,
    pub continue_on_error: bool,
    pub state: Mutex<State>,
}

/// Clears itself on drop, so `?` exits and cancellation can't leave a stale bar.
struct Spinner(ProgressBar);

impl Spinner {
    fn add(progress: &MultiProgress, name: &str) -> Spinner {
        let bar = progress.add(
            ProgressBar::new_spinner()
                .with_style(
                    ProgressStyle::with_template("{spinner:.cyan} {msg}: {pos} rows [{elapsed}]")
                        .expect("valid spinner template"),
                )
                .with_message(name.to_string()),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        Spinner(bar)
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.0.finish_and_clear();
    }
}

impl Sink<'_> {
    /// Loads `readers` collections at a time.
    pub async fn load(
        &self,
        scans: IndexMap<String, Scan>,
        readers: usize,
    ) -> Result<BTreeMap<String, LoadOutcome>, Error> {
        stream::iter(scans)
            .map(
                |(name, scan)| async move { Ok((name.clone(), self.load_one(&name, scan).await?)) },
            )
            .buffer_unordered(readers)
            .try_collect()
            .await
    }

    async fn load_one(&self, name: &str, scan: Scan) -> Result<LoadOutcome, Error> {
        let Scan { target, mut chunks } = scan;
        let started = Instant::now();
        let bar = Spinner::add(self.progress, name);
        let mut writer = BatchWriter {
            sink: self,
            name,
            collection: self.client.collection(name),
            // A resumed limit would be applied again from the cursor, and a partitioned run holds
            // rows behind every flush, so neither is checkpointed: they restart whole.
            checkpoint: target.limit.is_none() && target.partition.is_none(),
            batches: IndexMap::new(),
            bytes: 0,
            cursor: None,
            inflight: VecDeque::new(),
        };
        let mut outcome = LoadOutcome::default();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk?;
            for row in chunk.rows {
                match row.and_then(|record| build_row(&target, record)) {
                    Ok((partition, doc)) => {
                        outcome.rows += 1;
                        bar.0.inc(1);
                        writer.push(partition, doc).await?;
                    }
                    Err(e) if self.continue_on_error && matches!(e, Error::Doc { .. }) => {
                        crate::import::note(format!("{name}: skipped {e}"));
                        outcome.failed += 1;
                    }
                    Err(e) => return Err(e),
                }
            }
            writer.set_cursor(chunk.cursor);
        }
        writer.finish().await?;
        self.checkpoint(name, Mark::Done);
        outcome.elapsed = started.elapsed();
        Ok(outcome)
    }

    fn checkpoint(&self, name: &str, mark: Mark) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.cursors.insert(name.to_string(), mark);
        // A lost checkpoint costs a redo, never a skip.
        if let Err(e) = state.save() {
            tracing::warn!(%e, "cannot save run state");
        }
    }
}

/// One collection's write side: batches by size and partition, spawns each
/// batch's upsert under the run's budget, checkpoints cursors in flush order.
struct BatchWriter<'a> {
    sink: &'a Sink<'a>,
    name: &'a str,
    collection: CollectionClient,
    checkpoint: bool,
    /// Documents and their bytes, per partition.
    batches: IndexMap<Option<String>, (Vec<Document>, usize)>,
    /// Buffered across every partition.
    bytes: usize,
    /// Rows arrive before the cursor that covers them; it rides with the next flush.
    cursor: Option<Cursor>,
    /// A cursor is checkpointed once every preceding batch has completed.
    inflight: InflightBatches,
}

impl BatchWriter<'_> {
    async fn push(&mut self, partition: Option<String>, doc: Document) -> Result<(), Error> {
        let size = doc.encoded_len();
        self.bytes += size;
        let batch = self.batches.entry(partition.clone()).or_default();
        batch.0.push(doc);
        batch.1 += size;
        if batch.1 >= self.sink.batch_bytes {
            return self.flush(&partition).await;
        }
        if self.bytes >= self.sink.batch_bytes * BUFFERED_BATCHES {
            while let Some(partition) = self.batches.keys().next().cloned() {
                self.flush(&partition).await?;
            }
        }
        Ok(())
    }

    fn set_cursor(&mut self, cursor: Option<Cursor>) {
        if cursor.is_some() {
            self.cursor = cursor;
        }
    }

    async fn flush(&mut self, partition: &Option<String>) -> Result<(), Error> {
        let (docs, bytes) = match self.batches.swap_remove(partition) {
            Some(batch) => batch,
            None => return Ok(()),
        };
        self.bytes -= bytes;
        // Waits while the run is at `-c`; spawned upserts keep landing meanwhile.
        let permit = self
            .sink
            .budget
            .clone()
            .acquire_owned()
            .await
            .expect("budget is never closed");
        while self
            .inflight
            .front()
            .is_some_and(|(handle, _)| handle.is_finished())
        {
            self.complete_next().await?;
        }
        let collection = match partition {
            Some(partition) => self.collection.clone().partition(partition),
            None => self.collection.clone(),
        };
        self.inflight.push_back((
            tokio::spawn(async move {
                let _permit = permit;
                collection.upsert(docs).await?;
                Ok::<(), Error>(())
            }),
            self.cursor.take(),
        ));
        Ok(())
    }

    async fn complete_next(&mut self) -> Result<(), Error> {
        if let Some((handle, cursor)) = self.inflight.pop_front() {
            handle.await??;
            if let Some(cursor) = cursor.filter(|_| self.checkpoint) {
                self.sink.checkpoint(self.name, Mark::After(cursor));
            }
        }
        Ok(())
    }

    async fn finish(mut self) -> Result<(), Error> {
        while let Some(partition) = self.batches.keys().next().cloned() {
            self.flush(&partition).await?;
        }
        while !self.inflight.is_empty() {
            self.complete_next().await?;
        }
        Ok(())
    }
}
