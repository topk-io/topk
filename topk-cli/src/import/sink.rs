use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::{stream, Stream, StreamExt, TryStreamExt};
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use prost::Message;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use topk_rs::proto::v1::data::{Document, Value};
use topk_rs::Client;

use crate::client::DataClient;
use crate::config::Config;
use crate::import::ddl::{self, Schema};
use crate::import::decode::{self, id_string};
use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::{ChunkStream, Record, Source, Uri};
use crate::import::spec::{Spec, Target};
use crate::import::state::{Checkpoint, Destination, Mark, Run};
use crate::import::{ID, ID_PLACEHOLDER};

const SLOW_DOWN_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const SLOW_DOWN_INIT_BACKOFF: Duration = Duration::from_millis(250);
const SLOW_DOWN_MAX_BACKOFF: Duration = Duration::from_secs(10);
const OBJECT_CONCURRENCY: usize = 8;
/// Skipped rows named one by one before the rest are only counted.
const SKIP_NOTES: usize = 3;
/// Batches buffered across partitions before a full drain, which is what a checkpoint rides on.
const BUFFER_BATCHES: usize = 32;
struct Bar {
    bar: ProgressBar,
}

impl Bar {
    fn new(progress: &MultiProgress, name: &str, position: u64) -> Self {
        let bar = progress.add(
            ProgressBar::new_spinner()
                .with_style(
                    ProgressStyle::with_template("{spinner:.cyan} {msg}: {pos} rows [{elapsed}]")
                        .expect("valid spinner template"),
                )
                .with_message(name.to_string())
                .with_position(position),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        Self { bar }
    }
}

impl Drop for Bar {
    fn drop(&mut self) {
        self.bar.finish_and_clear();
    }
}

#[derive(Default, serde::Serialize)]
pub struct LoadOutcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
    #[serde(skip)]
    pub elapsed: Duration,
}

struct Skipped {
    name: String,
    failed: AtomicUsize,
}

impl Skipped {
    fn skip(&self, error: impl fmt::Display) {
        match self.failed.fetch_add(1, Ordering::Relaxed) {
            0..SKIP_NOTES => crate::import::note(format!("{}: skipped {error}", self.name)),
            SKIP_NOTES => crate::import::note(format!(
                "{}: skipping more failing rows; the summary counts them",
                self.name
            )),
            _ => {}
        }
    }
}

/// This row's partition, from the value of the `partition` column, and its document.
pub fn build_row(target: &Target, record: Record) -> Result<(Option<String>, Document), Error> {
    let fail = |id: Option<&str>, field: Option<&str>, source: Error| Error::Doc {
        id: id.map(str::to_string),
        field: field.map(str::to_string),
        source: Box::new(source),
    };
    let partition = if let Some(column) = target.partition.as_deref() {
        let value = record
            .iter()
            .find(|(key, _)| key == column)
            .map(|(_, value)| value)
            .filter(|value| value.as_null().is_none())
            .ok_or_else(|| {
                fail(
                    None,
                    Some(column),
                    Error::InvalidArgument("partition column is missing or null".to_string()),
                )
            })?;
        let partition = decode::text(value.clone()).map_err(|e| fail(None, Some(column), e))?;
        let mut chars = partition.chars();
        if partition.len() > 128
            || !matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
            || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        {
            return Err(fail(
                None,
                Some(column),
                Error::InvalidArgument(format!(
                    "partition value {partition:?} must start with a letter or digit, \
                     contain only letters, digits, `_` or `-`, and be at most 128 bytes"
                )),
            ));
        }
        Some(partition)
    } else {
        None
    };
    let id_column = target.id_column();
    let id = match record.iter().find(|(key, _)| key == id_column) {
        Some((_, value)) => id_string(id_column, value.clone())?,
        None => {
            let seen: Vec<_> = record.iter().map(|(key, _)| key.as_str()).collect();
            return Err(fail(
                None,
                Some(id_column),
                Error::InvalidArgument(format!(
                    "id column not present in this row, which has: {}",
                    seen.join(", ")
                )),
            ));
        }
    };
    // The spec is a whitelist; several fields may read one column, the id included.
    let mut doc = Document {
        fields: HashMap::with_capacity(target.fields.len() + 1),
    };
    for (name, field) in &target.fields {
        let missing = || {
            fail(
                Some(&id),
                Some(name),
                Error::InvalidArgument("required field is missing".to_string()),
            )
        };
        let value = match record.iter().find(|(key, _)| key == field.source(name)) {
            Some((_, value)) => value,
            None if field.required => return Err(missing()),
            None => continue,
        };
        let value = field
            .coerce(value.clone())
            .map_err(|e| fail(Some(&id), Some(name), e))?;
        if field.required && value.as_null().is_some() {
            return Err(missing());
        }
        doc.fields.insert(name.clone(), value);
    }

    doc.fields.insert(ID.to_string(), Value::string(id.clone()));
    let size = doc.encoded_len();
    if size > MAX_DOC_BYTES {
        return Err(fail(Some(&id), None, Error::Oversized(size)));
    }
    Ok((partition, doc))
}

pub fn documents(
    source: &Source,
    target: &Target,
) -> Result<impl Stream<Item = Result<(Option<String>, Document), Error>>, Error> {
    let chunks = source.scan(target, None)?;
    let target = target.clone();
    Ok(chunks
        .flat_map(|chunk| {
            stream::iter(match chunk {
                Ok(chunk) => chunk.rows,
                Err(e) => vec![Err(e)],
            })
        })
        .map(move |row| build_row(&target, row?)))
}

#[derive(clap::Args, Debug)]
pub struct WriteOptions {
    #[arg(
        long,
        help = "Skip documents that fail instead of stopping; exits non-zero if any did"
    )]
    pub continue_on_error: bool,
    #[arg(
        short = 'c',
        long,
        value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..=1024),
        default_value = "16",
        help = "Concurrent upserts in flight, budgeted across the whole run"
    )]
    pub concurrency: usize,
    #[arg(
        long,
        value_name = "SIZE",
        default_value = "1MiB",
        help = "Buffered document bytes per collection before flushing all partitions"
    )]
    pub batch_bytes: bytesize::ByteSize,
}

pub struct Import {
    pub run: Run,
    client: DataClient,
    source: Source,
    scans: Vec<(String, Target, ChunkStream, Option<Checkpoint>)>,
    pending: HashMap<String, Schema>,
}

impl Import {
    pub async fn new(
        config: &Config,
        source: Source,
        spec: Spec,
        destination: Destination,
    ) -> Result<Import, Error> {
        for (name, target) in &spec.collections {
            if target.id.as_deref() == Some(ID_PLACEHOLDER) {
                return Err(Error::MissingId(name.clone()));
            }
        }
        let run = Run::start(&source.locator, destination, spec).await?;
        Self::prepare(config, source, run).await
    }

    pub async fn resume(
        config: &Config,
        run: Run,
        supplied: Option<&Uri>,
    ) -> Result<Import, Error> {
        let source = run.source(supplied)?.build(config).await?;
        source.validate(run.pending(), None).await?;
        Self::prepare(config, source, run).await
    }

    async fn prepare(config: &Config, source: Source, run: Run) -> Result<Import, Error> {
        let client = DataClient::new(
            config,
            run.destination.project_id.clone(),
            &run.destination.region,
        )?;
        let scans = run
            .pending()
            .map(|(name, target)| {
                let mut target = target.clone();
                let resumed = match run.cursors.get(name) {
                    Some(Mark::After(checkpoint)) => {
                        target.limit = target.limit.map(|limit| limit - checkpoint.consumed);
                        Some(checkpoint.clone())
                    }
                    _ => None,
                };
                let chunks = source.scan(&target, resumed.as_ref().map(|c| c.cursor.clone()))?;
                Ok((name.clone(), target, chunks, resumed))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let pending = ddl::absent(&client, run.pending()).await?;
        Ok(Import {
            run,
            client,
            source,
            scans,
            pending,
        })
    }

    pub async fn execute(
        self,
        progress: &MultiProgress,
        options: &WriteOptions,
    ) -> Result<BTreeMap<String, LoadOutcome>, Error> {
        // An unwritable config dir costs the ability to resume, not the import.
        if let Err(e) = self.run.save() {
            crate::import::note(format!(
                "cannot save run state ({e}) — this run cannot be resumed"
            ));
        }
        let source = &self.source;
        let readers = self
            .scans
            .len()
            .min(OBJECT_CONCURRENCY)
            .min(source.concurrency_limit(self.run.pending().map(|(_, target)| target)))
            .max(1);
        let state = Arc::new(Mutex::new(self.run));
        let budget = Arc::new(Semaphore::new(options.concurrency));
        let client = &self.client;
        for (name, schema) in self.pending {
            match client.collections().create(&name, schema, None).await {
                Ok(_) | Err(topk_rs::Error::CollectionAlreadyExists) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let outcomes = stream::iter(self.scans)
            .map(|(name, target, mut chunks, resumed)| {
                let state = &state;
                let budget = &budget;
                async move {
                    let started = Instant::now();
                    let mut consumed = resumed.as_ref().map_or(0, |c| c.consumed);
                    let first = consumed;
                    let skipped = options.continue_on_error.then(|| {
                        Arc::new(Skipped {
                            name: name.clone(),
                            failed: AtomicUsize::new(0),
                        })
                    });
                    let bar = Bar::new(progress, &name, consumed);
                    let mut writer = BatchWriter {
                        client,
                        budget,
                        batch_bytes: options.batch_bytes.as_u64() as usize,
                        state,
                        name: &name,
                        bar,
                        batches: IndexMap::new(),
                        bytes: 0,
                        cursor: None,
                        inflight: VecDeque::new(),
                    };
                    while let Some(chunk) = chunks.next().await {
                        let chunk = chunk?;
                        consumed += chunk.rows.len() as u64;
                        for row in chunk.rows {
                            match row.and_then(|record| build_row(&target, record)) {
                                Ok((partition, doc)) => {
                                    writer.push(partition, doc).await?;
                                    writer.bar.bar.inc(1);
                                }
                                Err(e)
                                    if options.continue_on_error
                                        && matches!(e, Error::Doc { .. }) =>
                                {
                                    if let Some(skipped) = &skipped {
                                        skipped.skip(format!("{:#}", anyhow::Error::new(e)));
                                    }
                                    writer.bar.bar.inc(1);
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        if let Some(cursor) = chunk.cursor {
                            writer.checkpoint(Checkpoint { cursor, consumed });
                        }
                    }
                    writer.finish().await?;
                    let failed = skipped
                        .as_ref()
                        .map_or(0, |s| s.failed.load(Ordering::Relaxed));
                    state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .mark(&name, Mark::Done);
                    Ok::<_, Error>((
                        name,
                        LoadOutcome {
                            rows: (consumed - first) as usize - failed,
                            failed,
                            elapsed: started.elapsed(),
                        },
                    ))
                }
            })
            .buffer_unordered(readers)
            .try_collect()
            .await?;
        state.lock().unwrap_or_else(|e| e.into_inner()).remove();
        Ok(outcomes)
    }
}

/// One collection's write side: batches by size and partition, spawns each
/// batch's upsert under the run's budget, checkpoints cursors in flush order.
struct BatchWriter<'a> {
    client: &'a Client,
    budget: &'a Arc<Semaphore>,
    batch_bytes: usize,
    state: &'a Arc<Mutex<Run>>,
    name: &'a str,
    bar: Bar,
    batches: IndexMap<Option<String>, (usize, Vec<Document>)>,
    /// Buffered across every partition.
    bytes: usize,
    /// A source prefix is covered only by the last request of a full buffer drain.
    cursor: Option<Checkpoint>,
    /// A cursor is checkpointed once every preceding batch has completed.
    inflight: VecDeque<(JoinHandle<Result<(), Error>>, Option<Checkpoint>)>,
}

impl BatchWriter<'_> {
    fn checkpoint(&mut self, checkpoint: Checkpoint) {
        if !self.batches.is_empty() {
            self.cursor = Some(checkpoint);
        } else if let Some((_, after)) = self.inflight.back_mut() {
            *after = Some(checkpoint);
        } else {
            self.state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .mark(self.name, Mark::After(checkpoint));
        }
    }

    async fn push(&mut self, partition: Option<String>, doc: Document) -> Result<(), Error> {
        let size = doc.encoded_len();
        self.bytes += size;
        let entry = self.batches.entry(partition);
        let index = entry.index();
        let batch = entry.or_default();
        batch.0 += size;
        batch.1.push(doc);
        if self.bytes >= BUFFER_BATCHES * self.batch_bytes {
            while let Some((partition, (_, docs))) = self.batches.pop() {
                self.flush(partition, docs).await?;
            }
            self.bytes = 0;
        } else if batch.0 >= self.batch_bytes {
            if let Some((partition, (bytes, docs))) = self.batches.swap_remove_index(index) {
                self.bytes -= bytes;
                self.flush(partition, docs).await?;
            }
        }
        Ok(())
    }

    async fn flush(&mut self, partition: Option<String>, docs: Vec<Document>) -> Result<(), Error> {
        // Waits while the run is at `-c`; spawned upserts keep landing meanwhile.
        let permit = self
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
        let mut collection = self.client.collection(self.name);
        if let Some(partition) = partition {
            collection = collection.partition(partition);
        }
        let run = Arc::clone(self.state);
        self.inflight.push_back((
            tokio::spawn(async move {
                let _run = run;
                let _permit = permit;
                let started = Instant::now();
                let mut backoff = SLOW_DOWN_INIT_BACKOFF;
                loop {
                    match collection.upsert(docs.clone()).await {
                        Ok(_) => break,
                        Err(topk_rs::Error::SlowDown(_))
                            if started.elapsed() < SLOW_DOWN_TIMEOUT =>
                        {
                            tokio::time::sleep(backoff).await;
                            backoff = (backoff * 2).min(SLOW_DOWN_MAX_BACKOFF);
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
                Ok::<(), Error>(())
            }),
            if self.batches.is_empty() {
                self.cursor.take()
            } else {
                None
            },
        ));
        Ok(())
    }

    async fn complete_next(&mut self) -> Result<(), Error> {
        if let Some((handle, cursor)) = self.inflight.front_mut() {
            handle.await??;
            if let Some(cursor) = cursor.take() {
                self.state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .mark(self.name, Mark::After(cursor));
            }
            self.inflight.pop_front();
        }
        Ok(())
    }

    async fn finish(mut self) -> Result<(), Error> {
        while let Some((partition, (_, batch))) = self.batches.pop() {
            self.flush(partition, batch).await?;
        }
        while !self.inflight.is_empty() {
            self.complete_next().await?;
        }
        Ok(())
    }
}

impl Drop for BatchWriter<'_> {
    fn drop(&mut self) {
        for (handle, _) in &self.inflight {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{File, TryLockError};

    use topk_rs::ClientConfig;

    use super::*;

    #[tokio::test]
    async fn cancelled_writes_hold_lock() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("TOPK_IMPORT_STATE_DIR", dir.path());
        let run = Run::start(
            "",
            Destination {
                project_id: "p1".parse().unwrap(),
                region: "unused".into(),
            },
            toml::from_str("[c]\nfrom = 'unused'\n[c.fields]\ntitle = { type = 'text' }").unwrap(),
        )
        .await
        .unwrap();
        let lock = File::open(dir.path().join(format!("{}.lock", run.id))).unwrap();
        let state = Arc::new(Mutex::new(run));
        let client = Client::new(ClientConfig::new("unused", "unused"));
        let budget = Arc::new(Semaphore::new(1));
        let mut writer = BatchWriter {
            client: &client,
            budget: &budget,
            batch_bytes: 1024,
            state: &state,
            name: "c",
            bar: Bar::new(
                &MultiProgress::with_draw_target(indicatif::ProgressDrawTarget::hidden()),
                "c",
                0,
            ),
            batches: IndexMap::new(),
            bytes: 0,
            cursor: None,
            inflight: VecDeque::new(),
        };
        writer
            .flush(None, vec![topk_rs::doc!("_id" => "one", "title" => "book")])
            .await
            .unwrap();
        let task = writer.inflight[0].0.abort_handle();
        drop(writer);
        drop(state);
        assert!(!task.is_finished());
        assert!(matches!(lock.try_lock(), Err(TryLockError::WouldBlock)));
        while !task.is_finished() {
            tokio::task::yield_now().await;
        }
        lock.try_lock().unwrap();
    }
}
