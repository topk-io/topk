use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::{stream, Stream, StreamExt, TryStreamExt};
use indexmap::IndexMap;
use prost::Message;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use topk_rs::proto::v1::data::Document;
use topk_rs::Client;

use crate::import::ddl::{self, Schema};
use crate::import::error::Error;
use crate::import::source::Checkpoint;
use crate::import::source::{ReadEvent, ReadStream, Source};
use crate::import::spec::Target;
use crate::import::state::{Progress, Run};

const OBJECT_CONCURRENCY: usize = 8;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Outcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
    #[serde(skip)]
    pub elapsed: Duration,
}

pub enum Event {
    Progress {
        collection: String,
        written: usize,
    },
    Skipped {
        collection: String,
        error: Error,
    },
    Completed {
        collection: String,
        outcome: Outcome,
    },
}

#[derive(Debug)]
pub struct WriteOptions {
    pub continue_on_error: bool,
    pub concurrency: usize,
    pub batch_bytes: bytesize::ByteSize,
}

pub struct Import {
    run: Run,
    client: Client,
    source: Source,
    scans: Vec<(String, Target, ReadStream, u64)>,
    pending: HashMap<String, Schema>,
}

impl Import {
    pub fn run(&self) -> &Run {
        &self.run
    }

    pub async fn prepare(source: Source, client: Client, run: Run) -> Result<Self, Error> {
        run.state().validate()?;
        if source.locator() != run.state().source {
            return Err(Error::InvalidArgument(
                "source does not match the saved run".into(),
            ));
        }
        if client.config().region() != Some(run.state().destination.region.as_str()) {
            return Err(Error::InvalidArgument(
                "destination region does not match the saved run".into(),
            ));
        }
        let mut scans = Vec::new();
        for (name, target) in run.state().pending() {
            let resumed = match run.state().progress.get(name) {
                Some(Progress::After(checkpoint)) => Some(checkpoint),
                _ => None,
            };
            let chunks = source.scan(target, resumed).await?;
            scans.push((
                name.clone(),
                target.clone(),
                chunks,
                resumed.map_or(0, |c| c.consumed),
            ));
        }
        let pending = ddl::absent(&client, run.state().pending()).await?;
        Ok(Self {
            run,
            client,
            source,
            scans,
            pending,
        })
    }

    pub fn execute(self, options: WriteOptions) -> impl Stream<Item = Result<Event, Error>> {
        async_stream::try_stream! {
            if options.concurrency == 0 || options.concurrency > 1024 || options.batch_bytes.as_u64() == 0 {
                Err(Error::InvalidArgument("concurrency must be 1..=1024 and batch size must be positive".into()))?;
            }
            let buffer_bytes = options.batch_bytes.as_u64().checked_mul(options.concurrency as u64)
                .ok_or_else(|| Error::InvalidArgument("batch size times concurrency is too large".into()))?;
            if let Err(error) = self.run.save() {
                tracing::error!(%error, "cannot save run state — this run cannot be resumed");
            }
            let readers = self.scans.len().min(OBJECT_CONCURRENCY)
                .min(self.source.concurrency_limit(self.run.state().pending().map(|(_, target)| target))).max(1);
            let state = Arc::new(Mutex::new(self.run));
            let budget = Arc::new(Semaphore::new(options.concurrency));
            let client = &self.client;
            for (name, schema) in self.pending {
                match client.collections().create(&name, schema, None).await {
                    Ok(_) | Err(topk_rs::Error::CollectionAlreadyExists) => {}
                    Err(error) => Err(Error::from(error))?,
                }
            }
            let streams = stream::iter(self.scans).map(|(name, target, mut chunks, mut consumed)| -> futures::stream::BoxStream<'_, Result<Event, Error>> {
                let state = &state;
                let budget = &budget;
                let options = &options;
                async_stream::try_stream! {
                    let started = Instant::now();
                    let mut written = 0;
                    let mut failed = 0;
                    let mut batches = IndexMap::<Option<String>, Batch>::new();
                    let mut buffered = 0;
                    let mut inflight = JoinSet::new();
                    let mut checkpoints = Checkpoints::default();
                    let mut rows = Vec::<Result<crate::import::source::Record, Error>>::new().into_iter();
                    let mut chunk_id = 0;
                    let mut eof = false;
                    yield Event::Progress { collection: name.clone(), written };
                    loop {
                        if let Some(checkpoint) = checkpoints.ready() {
                            let saved = state.lock().unwrap_or_else(|e| e.into_inner()).mark(&name, Progress::After(checkpoint));
                            if let Err(error) = saved { tracing::error!(collection = %name, %error, "cannot save run state"); }
                        }
                        if eof && batches.is_empty() && inflight.is_empty() { break; }
                        let ready = batches.iter().enumerate()
                            .filter(|(_, (_, batch))| eof || buffered >= buffer_bytes || batch.bytes >= options.batch_bytes.as_u64())
                            .max_by_key(|(_, (_, batch))| batch.bytes)
                            .map(|(index, _)| index);
                        let next = tokio::select! {
                            biased;
                            completed = inflight.join_next(), if !inflight.is_empty() => Ready::Written(completed),
                            permit = budget.clone().acquire_owned(), if ready.is_some() && inflight.len() < options.concurrency => Ready::Permit(permit),
                            row = async { rows.next() }, if ready.is_none() && !rows.as_slice().is_empty() => Ready::Row(row),
                            event = chunks.try_next(), if ready.is_none() && rows.as_slice().is_empty() && !eof => Ready::Read(event),
                        };
                        match next {
                            Ready::Written(completed) => {
                                let outcome = completed
                                    .ok_or_else(|| Error::InvalidArgument("missing completed batch".into()))??;
                                written += outcome.written;
                                failed += outcome.skipped.len();
                                for error in outcome.skipped {
                                    yield Event::Skipped { collection: name.clone(), error };
                                }
                                yield Event::Progress { collection: name.clone(), written };
                                outcome.result?;
                                for (id, count) in outcome.chunks { checkpoints.complete(id, count)?; }
                            }
                            Ready::Permit(permit) => {
                                let permit = permit.map_err(|e| Error::InvalidArgument(e.to_string()))?;
                                if let Some((partition, batch)) = ready.and_then(|index| batches.swap_remove_index(index)) {
                                    buffered -= batch.bytes;
                                    let mut collection = client.collection(&name);
                                    if let Some(partition) = partition { collection = collection.partition(partition); }
                                    let run = Arc::clone(state);
                                    let continue_on_error = options.continue_on_error;
                                    inflight.spawn(async move {
                                        let _run = run;
                                        let _permit = permit;
                                        let docs = batch.docs;
                                        let mut outcome = BatchOutcome {
                                            chunks: batch.chunks,
                                            written: 0,
                                            skipped: Vec::new(),
                                            result: Ok(()),
                                        };
                                        let count = docs.len();
                                        let retry = continue_on_error.then(|| docs.clone());
                                        match collection.upsert(docs).await {
                                            Ok(_) => outcome.written = count,
                                            Err(error @ topk_rs::Error::DocumentValidationError(_)) if continue_on_error => {
                                                if count == 1 {
                                                    outcome.skipped.push(error.into());
                                                } else if let Some(docs) = retry {
                                                    for doc in docs {
                                                        match collection.upsert(vec![doc]).await {
                                                            Ok(_) => outcome.written += 1,
                                                            Err(error @ topk_rs::Error::DocumentValidationError(_)) => {
                                                                outcome.skipped.push(error.into())
                                                            }
                                                            Err(error) => {
                                                                outcome.result = Err(error.into());
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(error) => outcome.result = Err(error.into()),
                                        }
                                        outcome
                                    });
                                }
                            }
                            Ready::Row(row) => {
                                if let Some(row) = row {
                                    match row.and_then(|record| target.document(record)) {
                                        Ok((partition, doc)) => {
                                            let batch = batches.entry(partition).or_default();
                                            let size = doc.encoded_len() as u64;
                                            batch.bytes += size;
                                            buffered += size;
                                            batch.docs.push(doc);
                                            if let Some((_, count)) = batch.chunks.last_mut().filter(|(id, _)| *id == chunk_id) {
                                                *count += 1;
                                            } else {
                                                batch.chunks.push((chunk_id, 1));
                                            }
                                        }
                                        Err(error) if options.continue_on_error => {
                                            checkpoints.complete(chunk_id, 1)?;
                                            failed += 1;
                                            yield Event::Skipped { collection: name.clone(), error };
                                        }
                                        Err(error) => Err(error)?,
                                    }
                                }
                            }
                            Ready::Read(event) => {
                                match event? {
                                    Some(ReadEvent::Restarted) => {
                                        consumed = 0;
                                        tracing::warn!(collection = %name, "resume cursor expired, restarting from the beginning");
                                    }
                                    Some(ReadEvent::Chunk(chunk)) => {
                                        consumed += chunk.rows.len() as u64;
                                        chunk_id = checkpoints.push(chunk.rows.len(), chunk.cursor.map(|cursor| Checkpoint { cursor, consumed }));
                                        if chunk.rows.is_empty() { checkpoints.complete(chunk_id, 0)?; }
                                        rows = chunk.rows.into_iter();
                                    }
                                    None => eof = true,
                                }
                            }
                        }
                    }
                    let saved = state.lock().unwrap_or_else(|e| e.into_inner()).mark(&name, Progress::Done);
                    if let Err(error) = saved { tracing::error!(collection = %name, %error, "cannot save run state"); }
                    yield Event::Completed { collection: name, outcome: Outcome { rows: written, failed, elapsed: started.elapsed() } };
                }.boxed()
            }).flatten_unordered(readers);
            let mut streams = Box::pin(streams);
            while let Some(event) = streams.next().await {
                match event {
                    Ok(event) => yield event,
                    Err(error) => {
                        drop(streams);
                        Err(error)?;
                        return;
                    }
                }
            }
            let removed = state.lock().unwrap_or_else(|e| e.into_inner()).remove();
            if let Err(error) = removed { tracing::warn!(%error, "cannot remove run state"); }
        }
    }
}

enum Ready {
    Written(Option<Result<BatchOutcome, tokio::task::JoinError>>),
    Permit(Result<tokio::sync::OwnedSemaphorePermit, tokio::sync::AcquireError>),
    Row(Option<Result<crate::import::source::Record, Error>>),
    Read(Result<Option<ReadEvent>, Error>),
}

struct BatchOutcome {
    chunks: Vec<(u64, usize)>,
    written: usize,
    skipped: Vec<Error>,
    result: Result<(), Error>,
}

#[derive(Default)]
struct Batch {
    bytes: u64,
    docs: Vec<Document>,
    chunks: Vec<(u64, usize)>,
}

#[derive(Default)]
struct Checkpoints {
    next: u64,
    chunks: BTreeMap<u64, (usize, Option<Checkpoint>)>,
}

impl Checkpoints {
    fn push(&mut self, rows: usize, checkpoint: Option<Checkpoint>) -> u64 {
        let id = self.next;
        self.next += 1;
        self.chunks.insert(id, (rows, checkpoint));
        id
    }

    fn complete(&mut self, id: u64, rows: usize) -> Result<(), Error> {
        let chunk = self
            .chunks
            .get_mut(&id)
            .ok_or_else(|| Error::InvalidArgument("unknown source chunk".into()))?;
        chunk.0 = chunk
            .0
            .checked_sub(rows)
            .ok_or_else(|| Error::InvalidArgument("source chunk completed twice".into()))?;
        if chunk.0 != 0 {
            return Ok(());
        }
        let mut checkpoint = chunk.1.take();
        self.chunks.remove(&id);
        if let Some((&previous, (0, _))) = self.chunks.range(..id).next_back() {
            if let Some((_, before)) = self.chunks.remove(&previous) {
                checkpoint = checkpoint.or(before);
            }
        }
        let mut end = id;
        if let Some((&next, (0, _))) = self.chunks.range(id..).next() {
            if let Some((_, after)) = self.chunks.remove(&next) {
                checkpoint = after.or(checkpoint);
            }
            end = next;
        }
        self.chunks.insert(end, (0, checkpoint));
        Ok(())
    }

    fn ready(&mut self) -> Option<Checkpoint> {
        let mut checkpoint = None;
        while self
            .chunks
            .first_key_value()
            .is_some_and(|(_, (rows, _))| *rows == 0)
        {
            if let Some((_, (_, next))) = self.chunks.pop_first() {
                checkpoint = next.or(checkpoint);
            }
        }
        checkpoint
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{File, TryLockError};

    use futures::FutureExt;
    use rstest::rstest;
    use topk_rs::ClientConfig;

    use super::*;

    #[rstest]
    #[case::acknowledged_prefix(false)]
    #[case::failed_prefix(true)]
    fn unordered_completion_keeps_prefix(#[case] fatal: bool) {
        let mut checkpoints = Checkpoints::default();
        let first = checkpoints.push(
            2,
            Some(Checkpoint {
                cursor: crate::import::source::Cursor::Key("2".into()),
                consumed: 2,
            }),
        );
        let second = checkpoints.push(
            1,
            Some(Checkpoint {
                cursor: crate::import::source::Cursor::Key("3".into()),
                consumed: 3,
            }),
        );
        checkpoints.complete(second, 1).unwrap();
        assert!(checkpoints.ready().is_none());
        checkpoints.complete(first, 1).unwrap();
        assert!(checkpoints.ready().is_none());
        if !fatal {
            checkpoints.complete(first, 1).unwrap();
            assert_eq!(checkpoints.ready().unwrap().consumed, 3);
            assert!(checkpoints.chunks.is_empty());
        }
    }

    #[test]
    fn stalled_chunk_bounds_checkpoints() {
        let mut checkpoints = Checkpoints::default();
        let first = checkpoints.push(1, None);
        for n in 1..10_000 {
            let next = checkpoints.push(
                1,
                Some(Checkpoint {
                    cursor: crate::import::source::Cursor::Key(n.to_string()),
                    consumed: n,
                }),
            );
            checkpoints.complete(next, 1).unwrap();
            assert!(checkpoints.ready().is_none());
            assert_eq!(checkpoints.chunks.len(), 2);
        }
        checkpoints.complete(first, 1).unwrap();
        assert_eq!(checkpoints.ready().unwrap().consumed, 9999);
    }

    #[test]
    fn sparse_chunks_keep_last_cursor() {
        let mut checkpoints = Checkpoints::default();
        let first = checkpoints.push(1, None);
        let second = checkpoints.push(
            1,
            Some(Checkpoint {
                cursor: crate::import::source::Cursor::Key("2".into()),
                consumed: 2,
            }),
        );
        let third = checkpoints.push(1, None);
        checkpoints.complete(third, 1).unwrap();
        checkpoints.complete(first, 1).unwrap();
        assert!(checkpoints.ready().is_none());
        checkpoints.complete(second, 1).unwrap();
        assert_eq!(checkpoints.ready().unwrap().consumed, 2);
    }

    #[tokio::test]
    async fn cancelled_writes_hold_lock() {
        let dir = tempfile::tempdir().unwrap();
        let spec: crate::import::spec::Spec =
            toml::from_str("[c]\nfrom = 'unused'\n[c.fields]\ntitle = { type = 'text' }").unwrap();
        let target = spec.collections["c"].clone();
        let run = Run::start(
            dir.path(),
            crate::import::state::State::new(
                String::new(),
                crate::scope::Scope {
                    project_id: "p1".parse().unwrap(),
                    region: "unused".into(),
                },
                spec,
            ),
        )
        .await
        .unwrap();
        let lock = File::open(dir.path().join(format!("{}.lock", run.id))).unwrap();
        let chunks = stream::iter([Ok(ReadEvent::Chunk(crate::import::source::Chunk {
            rows: vec![Ok(vec![
                ("_id".into(), "one".into()),
                ("title".into(), "book".into()),
            ])],
            cursor: None,
        }))])
        .boxed();
        let import = Import {
            run,
            client: Client::new(ClientConfig::new("unused", "unused")),
            source: crate::import::source::Uri::Files(None)
                .connect()
                .await
                .unwrap(),
            scans: vec![("c".into(), target, chunks, 0)],
            pending: HashMap::new(),
        };
        let mut execution = Box::pin(import.execute(WriteOptions {
            continue_on_error: false,
            concurrency: 1,
            batch_bytes: bytesize::ByteSize(1),
        }));
        assert!(matches!(
            execution.try_next().await.unwrap(),
            Some(Event::Progress { written: 0, .. })
        ));
        assert!(execution.try_next().now_or_never().is_none());
        drop(execution);
        assert!(matches!(lock.try_lock(), Err(TryLockError::WouldBlock)));
        tokio::time::timeout(Duration::from_secs(1), async {
            while matches!(lock.try_lock(), Err(TryLockError::WouldBlock)) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[rstest]
    #[case::row_error(false)]
    #[case::stream_error(true)]
    #[tokio::test]
    async fn source_error_boundary(#[case] stream_error: bool) {
        let dir = tempfile::tempdir().unwrap();
        let spec: crate::import::spec::Spec =
            toml::from_str("[c]\nfrom = 'unused'\n[c.fields]\ntitle = { type = 'text' }").unwrap();
        let target = spec.collections["c"].clone();
        let run = Run::start(
            dir.path(),
            crate::import::state::State::new(
                String::new(),
                crate::scope::Scope {
                    project_id: "p1".parse().unwrap(),
                    region: "unused".into(),
                },
                spec,
            ),
        )
        .await
        .unwrap();
        let id = run.id;
        let chunks = async_stream::try_stream! {
            yield ReadEvent::Chunk(crate::import::source::Chunk {
                rows: vec![Err(Error::CannotCoerce("int"))],
                cursor: Some(crate::import::source::Cursor::Key("1".into())),
            });
            if stream_error {
                Err(Error::InvalidArgument("source stream failed".into()))?;
            }
        }
        .boxed();
        let import = Import {
            run,
            client: Client::new(ClientConfig::new("unused", "unused")),
            source: crate::import::source::Uri::Files(None)
                .connect()
                .await
                .unwrap(),
            scans: vec![("c".into(), target, chunks, 0)],
            pending: HashMap::new(),
        };
        let mut execution = Box::pin(import.execute(WriteOptions {
            continue_on_error: true,
            concurrency: 1,
            batch_bytes: bytesize::ByteSize(1024),
        }));
        assert!(matches!(
            execution.try_next().await.unwrap(),
            Some(Event::Progress { written: 0, .. })
        ));
        assert!(matches!(
            execution.try_next().await.unwrap(),
            Some(Event::Skipped { .. })
        ));
        if stream_error {
            assert!(
                matches!(execution.try_next().await, Err(Error::InvalidArgument(message)) if message == "source stream failed")
            );
            drop(execution);
            let run = Run::load(dir.path(), id).await.unwrap();
            assert!(matches!(&run.state().progress["c"], Progress::After(c) if c.consumed == 1));
        } else {
            assert!(
                matches!(execution.try_next().await.unwrap(), Some(Event::Completed { outcome, .. }) if outcome.rows == 0 && outcome.failed == 1)
            );
            assert!(execution.try_next().await.unwrap().is_none());
            assert!(!dir.path().join(format!("{id}.toml")).exists());
        }
    }
}
