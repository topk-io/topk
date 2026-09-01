use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{stream, Stream, StreamExt};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use prost::Message;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use topk_rs::proto::v1::data::{Document, Value};
use topk_rs::{Client, CollectionClient};

use crate::import::decode::id_string;
use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::{Record, Scan};
use crate::import::spec::Target;
use crate::import::ID;

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
    let id_column = target.id.as_deref().unwrap_or(ID);
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
    for (key, value) in record {
        for (name, field) in target
            .fields
            .iter()
            .filter(|(name, field)| field.column(name) == key)
        {
            let value = field
                .coerce(value.clone())
                .map_err(|e| fail(Some(name), e))?;
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
        return Err(fail(None, Error::Oversized(size)));
    }
    Ok(doc)
}

pub async fn document_stream(
    scan: Scan,
) -> Result<impl Stream<Item = Result<Document, Error>>, Error> {
    let chunks = scan.chunk_stream().await?;
    let target = scan.target;
    Ok(chunks
        .flat_map(|chunk| stream::iter(chunk.map_or_else(|e| vec![Err(e)], |chunk| chunk.rows)))
        .map(move |row| build_document(&target, row?)))
}

/// Batches in flush order, each with the source cursor it completes.
type InflightBatches = VecDeque<(JoinHandle<Result<(), Error>>, Option<String>)>;

/// Collections load concurrently, as many as the source can serve at once;
/// their upserts share one budget of `-c` in flight across the run.
pub struct Sink<'a> {
    pub client: &'a Client,
    pub progress: &'a MultiProgress,
    pub budget: Arc<Semaphore>,
    pub batch_bytes: usize,
    pub continue_on_error: bool,
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
    /// Called with a source cursor once every row up to it has been upserted.
    pub async fn load(
        &self,
        scan: &Scan,
        checkpoint: impl FnMut(&str),
    ) -> Result<LoadOutcome, Error> {
        let started = Instant::now();
        let bar = Spinner::add(self.progress, &scan.name);
        let name = &scan.name;
        let target = &scan.target;
        let mut collection = self.client.collection(name);
        if let Some(partition) = &target.partition {
            collection = collection.partition(partition);
        }
        let mut chunks = scan.chunk_stream().await?;
        let mut writer = BatchWriter {
            sink: self,
            collection,
            checkpoint,
            batch: Vec::new(),
            bytes: 0,
            cursor: None,
            inflight: VecDeque::new(),
        };
        let mut outcome = LoadOutcome::default();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk?;
            for row in chunk.rows {
                match row.and_then(|record| build_document(target, record)) {
                    Ok(doc) => {
                        outcome.rows += 1;
                        bar.0.inc(1);
                        writer.push(doc).await?;
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
        outcome.elapsed = started.elapsed();
        Ok(outcome)
    }
}

/// One collection's write side: batches by size, spawns each batch's upsert
/// under the run's budget, checkpoints cursors in flush order.
struct BatchWriter<'a, F: FnMut(&str)> {
    sink: &'a Sink<'a>,
    collection: CollectionClient,
    checkpoint: F,
    batch: Vec<Document>,
    bytes: usize,
    /// Rows arrive before the cursor that covers them; it rides with the next flush.
    cursor: Option<String>,
    /// A cursor is checkpointed once every preceding batch has completed.
    inflight: InflightBatches,
}

impl<F: FnMut(&str)> BatchWriter<'_, F> {
    async fn push(&mut self, doc: Document) -> Result<(), Error> {
        self.bytes += doc.encoded_len();
        self.batch.push(doc);
        if self.bytes >= self.sink.batch_bytes {
            self.flush().await?;
        }
        Ok(())
    }

    fn set_cursor(&mut self, cursor: Option<String>) {
        if cursor.is_some() {
            self.cursor = cursor;
        }
    }

    async fn flush(&mut self) -> Result<(), Error> {
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
        let collection = self.collection.clone();
        let docs = mem::take(&mut self.batch);
        self.bytes = 0;
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
            if let Some(cursor) = cursor {
                (self.checkpoint)(&cursor);
            }
        }
        Ok(())
    }

    async fn finish(mut self) -> Result<(), Error> {
        if !self.batch.is_empty() {
            self.flush().await?;
        }
        while !self.inflight.is_empty() {
            self.complete_next().await?;
        }
        Ok(())
    }
}
