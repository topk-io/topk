use std::collections::VecDeque;
use std::mem;
use std::sync::Arc;

use futures::{stream, Stream, StreamExt};
use indicatif::ProgressBar;
use prost::Message;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use topk_rs::proto::v1::data::{Document, Value};
use topk_rs::{Client, CollectionClient};

use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::{Record, Scan};
use crate::import::spec::Target;
use crate::import::value::id_string;
use crate::import::ID;

#[derive(Default, serde::Serialize)]
pub struct Outcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
}

/// The document a target asks for, built from one source row.
pub fn document(target: &Target, record: Record) -> Result<Document, Error> {
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
            .filter(|(name, field)| field.from.as_deref().unwrap_or(name.as_str()) == key)
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

pub async fn documents(scan: Scan) -> Result<impl Stream<Item = Result<Document, Error>>, Error> {
    let chunks = scan.stream().await?;
    let target = scan.target;
    Ok(chunks
        .flat_map(|chunk| stream::iter(chunk.map_or_else(|e| vec![Err(e)], |chunk| chunk.rows)))
        .map(move |row| document(&target, row?)))
}

/// Batches in flush order, each with the source mark it completes.
type Inflight = VecDeque<(JoinHandle<Result<(), Error>>, Option<String>)>;

/// Collections load concurrently, as many as the source can serve at once;
/// their upserts share one budget of `-c` in flight across the run.
pub struct Sink<'a> {
    pub client: &'a Client,
    pub progress: &'a ProgressBar,
    pub budget: Arc<Semaphore>,
    pub batch_bytes: usize,
    pub continue_on_error: bool,
}

impl Sink<'_> {
    /// Called with a source mark once every row up to it has been upserted.
    pub async fn load(&self, scan: &Scan, checkpoint: impl FnMut(&str)) -> Result<Outcome, Error> {
        let name = &scan.name;
        let target = &scan.target;
        let mut collection = self.client.collection(name);
        if let Some(partition) = &target.partition {
            collection = collection.partition(partition);
        }
        let mut chunks = scan.stream().await?;
        let mut writer = Writer {
            sink: self,
            collection,
            checkpoint,
            batch: Vec::new(),
            bytes: 0,
            pending: None,
            inflight: VecDeque::new(),
        };
        let mut outcome = Outcome::default();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk?;
            for row in chunk.rows {
                match row.and_then(|record| document(target, record)) {
                    Ok(doc) => {
                        outcome.rows += 1;
                        self.progress.inc(1);
                        writer.push(doc).await?;
                    }
                    Err(e) if self.continue_on_error && matches!(e, Error::Doc { .. }) => {
                        eprintln!("{name}: skipped {e}");
                        outcome.failed += 1;
                    }
                    Err(e) => return Err(e),
                }
            }
            writer.mark(chunk.mark);
        }
        writer.finish().await?;
        Ok(outcome)
    }
}

/// One collection's write side: batches by size, spawns each batch's upsert
/// under the run's budget, checkpoints marks in flush order.
struct Writer<'a, F: FnMut(&str)> {
    sink: &'a Sink<'a>,
    collection: CollectionClient,
    checkpoint: F,
    batch: Vec<Document>,
    bytes: usize,
    /// Rows arrive before the mark that covers them; it rides with the next flush.
    pending: Option<String>,
    /// A mark is checkpointed once every batch up to its own has landed,
    /// whatever order they land in.
    inflight: Inflight,
}

impl<F: FnMut(&str)> Writer<'_, F> {
    async fn push(&mut self, doc: Document) -> Result<(), Error> {
        self.bytes += doc.encoded_len();
        self.batch.push(doc);
        if self.bytes >= self.sink.batch_bytes {
            self.flush().await?;
        }
        Ok(())
    }

    fn mark(&mut self, mark: Option<String>) {
        if mark.is_some() {
            self.pending = mark;
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
            self.land().await?;
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
            self.pending.take(),
        ));
        Ok(())
    }

    async fn land(&mut self) -> Result<(), Error> {
        if let Some((handle, mark)) = self.inflight.pop_front() {
            handle.await??;
            if let Some(mark) = mark {
                (self.checkpoint)(&mark);
            }
        }
        Ok(())
    }

    async fn finish(mut self) -> Result<(), Error> {
        if !self.batch.is_empty() {
            self.flush().await?;
        }
        while !self.inflight.is_empty() {
            self.land().await?;
        }
        Ok(())
    }
}
