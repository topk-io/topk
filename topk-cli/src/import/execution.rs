use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::{Stream, TryStreamExt};
use indexmap::IndexMap;
use prost::Message;
use rand::Rng;
use tokio::task::JoinSet;
use topk_rs::client::retry::RetryConfig;
use topk_rs::proto::v1::data::Document;
use topk_rs::{Client, CollectionClient};
use tracing::{debug, info, warn};

use crate::import::ddl;
use crate::import::error::Error;
use crate::import::plan::Target;
use crate::import::source::{Cursor, ReadEvent, ReadStream, Resume, Source};
use crate::import::state::{Progress, Run};

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Outcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
    #[serde(skip)]
    pub elapsed: Duration,
    #[serde(skip)]
    pub bytes: u64,
    /// Source rows consumed before this invocation, by the run it resumed.
    pub resumed_after: u64,
}

pub enum Event {
    Progress {
        written: usize,
        bytes: u64,
    },
    /// `row` counts from 1 in the file `part` when the source is several files, and in source
    /// order otherwise; a destination rejection has none.
    Skipped {
        row: Option<u64>,
        part: Option<String>,
        error: Error,
    },
    Completed {
        outcome: Outcome,
    },
}

/// Saved with the run, so a resume reads the way the run started.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ReadOptions {
    /// Bytes for open file readers and upserts in flight; half of the machine's when unset.
    pub max_memory: Option<u64>,
}

/// Saved with the run, so a resume writes the way the run started.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WriteOptions {
    pub continue_on_error: bool,
    /// Most upserts in flight; unset takes what the memory budget allows.
    pub concurrency: Option<usize>,
    pub batch_bytes: u64,
}

/// The window opens at `start` and grows up to `max` unless the run names a concurrency.
/// Until the first throttle it doubles after each window of responses. After it, a probe
/// adds a fraction `grow`, is judged over `rounds` windows and stays when throughput rose by
/// the fraction `gain`. A throttle retains `cut` of the window.
#[derive(Clone, Copy, Debug)]
pub struct Aimd {
    pub start: usize,
    pub max: usize,
    pub cut: f64,
    pub grow: f64,
    pub gain: f64,
    pub rounds: usize,
}

pub struct Import {
    run: Run,
    client: Client,
    scan: ReadStream,
    /// Chunks mapped to documents at once.
    mappers: usize,
    create: bool,
}

impl Import {
    pub fn run(&self) -> &Run {
        &self.run
    }

    pub async fn start(
        source: Source,
        client: Client,
        run: Run,
        create: bool,
    ) -> Result<Self, Error> {
        Self::validate(&source, &client, &run)?;
        let resumed = match &run.state().progress {
            Some(Progress::After(checkpoint)) => Some(checkpoint),
            _ => None,
        };
        let scan = source.scan(&run.state().plan.target, resumed).await?;
        Ok(Self {
            run,
            client,
            scan,
            mappers: source.readers.count,
            create,
        })
    }

    pub async fn resume(source: Source, client: Client, run: Run) -> Result<Self, Error> {
        Self::validate(&source, &client, &run)?;
        let plan = &run.state().plan;
        let create = match client.collections().get(&plan.collection).await {
            Ok(collection) => {
                plan.target
                    .validate_schema(&plan.collection, &collection.schema)?;
                false
            }
            Err(topk_rs::Error::CollectionNotFound) => true,
            Err(error) => return Err(error.into()),
        };
        Self::start(source, client, run, create).await
    }

    fn validate(source: &Source, client: &Client, run: &Run) -> Result<(), Error> {
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
        Ok(())
    }

    /// Called before the first write, so a run that fails before writing leaves no collection.
    async fn create(client: &Client, name: &str, target: &Target) -> Result<(), Error> {
        let schema: ddl::Schema = target
            .fields
            .iter()
            .map(|(name, field)| (name.clone(), field.schema.clone()))
            .collect();
        info!(collection = %name, "creating collection");
        match client.collections().create(name, schema, None).await {
            Ok(_) => Ok(()),
            Err(topk_rs::Error::CollectionAlreadyExists) => {
                let existing = client.collections().get(name).await?;
                target.validate_schema(name, &existing.schema)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub fn execute(self, aimd: Aimd) -> impl Stream<Item = Result<Event, Error>> {
        async_stream::try_stream! {
            let options = self.run.state().write.clone();
            let name = self.run.state().plan.collection.clone();
            let target = self.run.state().plan.target.clone();
            let client = &self.client;
            let retry = client.config().retry_config().clone();
            let upserts = Client::new(client.config().clone().with_retry_config(RetryConfig { max_retries: 0, ..retry.clone() }));
            let concurrency = options.concurrency.unwrap_or(aimd.max);
            let window = Arc::new(Mutex::new(Window::new(concurrency, aimd)));
            let consumed = match &self.run.state().progress {
                Some(Progress::After(checkpoint)) => checkpoint.consumed,
                _ => 0,
            };
            info!(run = %self.run.id, collection = %name, consumed, ?options, concurrency, mappers = self.mappers, "import started");
            let mut create = self.create;
            let state = Arc::new(Mutex::new(self.run));
            let mut feed = Box::pin(Self::feed(self.scan, target.clone(), &options, concurrency, consumed, self.mappers));
            let started = Instant::now();
            let mut written = 0;
            let mut bytes = 0;
            let mut failed = 0;
            let mut inflight = JoinSet::new();
            let mut positions = Positions { resumed: consumed, ..Default::default() };
            let mut eof = false;
            let mut pending = None;
            yield Event::Progress { written, bytes };
            loop {
                if let Some(resume) = positions.ready() {
                    state.lock().unwrap_or_else(|e| e.into_inner()).mark(Progress::After(resume))?;
                }
                if eof && inflight.is_empty() { break; }
                let (admits, probe) = {
                    let window = lock(&window);
                    (window.admits(inflight.len()), window.limit < window.max || window.previous.is_some())
                };
                let next = if admits && pending.is_some() {
                    Next::Feed(Ok(pending.take()))
                } else {
                    tokio::select! {
                        biased;
                        completed = inflight.join_next(), if !inflight.is_empty() => Next::Written(completed),
                        next = feed.try_next(), if !eof && pending.is_none() && (admits || probe) => Next::Feed(next),
                    }
                };
                match next {
                    Next::Written(completed) => {
                        let outcome = completed
                            .ok_or_else(|| Error::InvalidArgument("missing completed batch".into()))??;
                        written += outcome.written;
                        bytes += outcome.bytes;
                        failed += outcome.skipped.len();
                        for error in outcome.skipped {
                            yield Event::Skipped { row: None, part: None, error };
                        }
                        yield Event::Progress { written, bytes };
                        if let Some(error) = outcome.error { Err(error)?; }
                        for (id, count) in outcome.chunks { positions.complete(id, count)?; }
                    }
                    Next::Feed(message) => match message? {
                        None => eof = true,
                        Some(Feed::Chunk { rows, cursor, last }) => {
                            let id = positions.push(rows, cursor, last);
                            if rows == 0 { positions.complete(id, 0)?; }
                        }
                        Some(Feed::Restarted) => positions.restart(),
                        Some(Feed::Skipped { chunk, row, part, error }) => {
                            positions.complete(chunk, 1)?;
                            failed += 1;
                            let part = part.filter(|part| *part != target.from.value());
                            yield Event::Skipped { row: Some(row), part, error };
                        }
                        Some(Feed::Batch { partition, batch }) => {
                            let slot = {
                                let mut window = lock(&window);
                                if !window.admits(inflight.len()) {
                                    window.backlogged();
                                    pending = Some(Feed::Batch { partition, batch });
                                    continue;
                                }
                                window.slot()
                            };
                            if create {
                                Self::create(client, &name, &target).await?;
                                create = false;
                            }
                            let mut collection = upserts.collection(&name);
                            if let Some(partition) = partition { collection = collection.partition(partition); }
                            inflight.spawn(Self::write(collection, batch, options.continue_on_error, retry.clone(), Arc::clone(&window), slot, Arc::clone(&state)));
                        }
                    },
                }
            }
            if create && failed == 0 {
                Self::create(client, &name, &target).await?;
            }
            state.lock().unwrap_or_else(|e| e.into_inner()).mark(Progress::Done)?;
            yield Event::Completed { outcome: Outcome { rows: written, failed, bytes, resumed_after: consumed, elapsed: started.elapsed() } };
            let removed = state.lock().unwrap_or_else(|e| e.into_inner()).remove();
            if let Err(error) = removed { warn!(%error, "cannot remove run state"); }
        }
    }

    /// Documents batched per partition. A batch goes when full, once `window` bytes were batched
    /// since its first document, or at the end; this bounds both buffered bytes and checkpoint lag.
    /// Up to `mappers` chunks become documents at once and are batched as they finish, so one
    /// mapper keeps source order.
    fn feed(
        mut chunks: ReadStream,
        target: Target,
        options: &WriteOptions,
        concurrency: usize,
        mut consumed: u64,
        mappers: usize,
    ) -> impl Stream<Item = Result<Feed, Error>> {
        let batch_bytes = options.batch_bytes;
        let window = batch_bytes.saturating_mul(concurrency as u64);
        let continue_on_error = options.continue_on_error;
        async_stream::try_stream! {
            let target = Arc::new(target);
            let mut batches = IndexMap::<Option<String>, Batch>::new();
            // Open batches, oldest first; an entry outlives a batch that went when full.
            let mut opened = VecDeque::<(u64, Option<String>)>::new();
            let mut batched = 0;
            let mut chunk = 0;
            let mut mapping = JoinSet::new();
            let mut eof = false;
            while !eof || !mapping.is_empty() {
                let next = tokio::select! {
                    biased;
                    mapped = mapping.join_next(), if !mapping.is_empty() => Fed::Mapped(mapped),
                    event = chunks.try_next(), if !eof && mapping.len() < mappers => Fed::Read(event),
                };
                let mapped: Mapped = match next {
                    Fed::Mapped(mapped) => mapped
                        .ok_or_else(|| Error::InvalidArgument("missing mapped chunk".into()))??,
                    Fed::Read(event) => {
                        match event? {
                            None => eof = true,
                            Some(ReadEvent::Restarted) => {
                                consumed = 0;
                                warn!("resume cursor expired, restarting from the beginning");
                                yield Feed::Restarted;
                            }
                            Some(ReadEvent::Chunk(read)) => {
                                let rows = read.rows.len();
                                let (part, first) = match &read.cursor {
                                    Some(Cursor::Offset { part, rows: end }) => (Some(part.clone()), end.saturating_sub(rows as u64)),
                                    _ => (None, consumed),
                                };
                                consumed += rows as u64;
                                yield Feed::Chunk { rows, cursor: read.cursor, last: read.last };
                                if rows != 0 {
                                    let (id, target) = (chunk, Arc::clone(&target));
                                    mapping.spawn_blocking(move || Mapped {
                                        chunk: id,
                                        part,
                                        first,
                                        rows: read.rows.into_iter().map(|row| {
                                            let (partition, doc) = target.document(row?)?;
                                            Ok((partition, doc.encoded_len() as u64, doc))
                                        }).collect(),
                                    });
                                }
                                chunk += 1;
                            }
                        }
                        continue;
                    }
                };
                for (index, row) in mapped.rows.into_iter().enumerate() {
                    match row {
                        Ok((partition, size, doc)) => {
                            let slot = match batches.get_index_of(&partition) {
                                Some(slot) => slot,
                                None => {
                                    opened.push_back((batched, partition.clone()));
                                    batches.insert_full(partition, Batch { opened: batched, ..Default::default() }).0
                                }
                            };
                            let batch = &mut batches[slot];
                            batch.bytes += size;
                            batch.docs.push(doc);
                            match batch.chunks.last_mut() {
                                Some((id, count)) if *id == mapped.chunk => *count += 1,
                                _ => batch.chunks.push((mapped.chunk, 1)),
                            }
                            batched += size;
                            if batch.bytes >= batch_bytes {
                                if let Some((partition, batch)) = batches.swap_remove_index(slot) {
                                    yield Feed::Batch { partition, batch };
                                }
                            }
                        }
                        Err(error) if continue_on_error => {
                            yield Feed::Skipped { chunk: mapped.chunk, row: mapped.first + index as u64 + 1, part: mapped.part.clone(), error };
                        }
                        Err(error) => Err(error)?,
                    }
                    while opened.front().is_some_and(|(since, _)| batched - since >= window) {
                        if let Some((since, partition)) = opened.pop_front() {
                            if batches.get(&partition).is_some_and(|batch| batch.opened == since) {
                                if let Some(batch) = batches.swap_remove(&partition) {
                                    yield Feed::Batch { partition, batch };
                                }
                            }
                        }
                    }
                }
            }
            for (partition, batch) in batches {
                yield Feed::Batch { partition, batch };
            }
        }
    }

    /// `_run` keeps the run locked until the write ends, even when the import is dropped.
    async fn write(
        collection: CollectionClient,
        batch: Batch,
        continue_on_error: bool,
        retry: RetryConfig,
        window: Arc<Mutex<Window>>,
        mut slot: Slot,
        _run: Arc<Mutex<Run>>,
    ) -> BatchOutcome {
        let mut outcome = BatchOutcome {
            chunks: batch.chunks,
            ..Default::default()
        };
        let count = batch.docs.len();
        match Self::upsert(
            &collection,
            &batch.docs,
            &retry,
            &window,
            &mut slot,
            Some(batch.bytes),
        )
        .await
        {
            Ok(()) => {
                outcome.written = count;
                outcome.bytes = batch.bytes;
            }
            Err(error @ topk_rs::Error::DocumentValidationError(_))
                if continue_on_error && count == 1 =>
            {
                outcome.skipped.push(error.into())
            }
            Err(topk_rs::Error::DocumentValidationError(_)) if continue_on_error => {
                debug!(count, "retrying rejected batch individually");
                // One batch must not grow the window once per document.
                for doc in batch.docs {
                    let bytes = doc.encoded_len() as u64;
                    match Self::upsert(&collection, &[doc], &retry, &window, &mut slot, None).await
                    {
                        Ok(()) => {
                            outcome.written += 1;
                            outcome.bytes += bytes;
                        }
                        Err(error @ topk_rs::Error::DocumentValidationError(_)) => {
                            outcome.skipped.push(error.into())
                        }
                        Err(error) => {
                            outcome.error = Some(error.into());
                            break;
                        }
                    }
                }
            }
            Err(error) => outcome.error = Some(error.into()),
        }
        outcome
    }

    /// Retries a throttled or transient failure until the retry timeout; a throttle also cuts the
    /// window. The slot stays taken while it waits.
    async fn upsert(
        collection: &CollectionClient,
        docs: &[Document],
        retry: &RetryConfig,
        window: &Mutex<Window>,
        slot: &mut Slot,
        bytes: Option<u64>,
    ) -> Result<(), topk_rs::Error> {
        let started = Instant::now();
        let mut delay = Duration::ZERO;
        loop {
            let error = match collection.upsert(docs.to_vec()).await {
                Ok(_) => {
                    if let Some(bytes) = bytes {
                        lock(window).success(*slot, bytes, Instant::now());
                    }
                    return Ok(());
                }
                Err(error @ (topk_rs::Error::SlowDown(_) | topk_rs::Error::QuotaExceeded(_))) => {
                    lock(window).throttled(slot);
                    error
                }
                Err(error) if error.is_retryable() => error,
                Err(error) => return Err(error),
            };
            if started.elapsed() >= retry.timeout {
                return Err(error);
            }
            let base = retry.backoff.init_backoff;
            delay = rand::thread_rng()
                .gen_range(base..=3 * delay.max(base))
                .min(retry.backoff.max_backoff);
            debug!(%error, ?delay, "retrying upsert");
            tokio::time::sleep(delay).await;
        }
    }
}

struct Window {
    limit: usize,
    max: usize,
    aimd: Aimd,
    cuts: u64,
    settle: usize,
    waiting: bool,
    generation: u64,
    first_ack: Option<Instant>,
    bytes: u64,
    acks: usize,
    previous: Option<(usize, f64)>,
}

impl Window {
    fn new(max: usize, aimd: Aimd) -> Self {
        Self {
            limit: aimd.start.min(max),
            max,
            aimd,
            cuts: 0,
            settle: 0,
            waiting: false,
            generation: 0,
            first_ack: None,
            bytes: 0,
            acks: 0,
            previous: None,
        }
    }

    fn admits(&self, inflight: usize) -> bool {
        inflight < self.limit
    }

    fn backlogged(&mut self) {
        self.waiting = true;
    }

    fn slot(&self) -> Slot {
        Slot {
            generation: self.generation,
            cut: self.cuts,
        }
    }

    fn reset(&mut self) {
        self.generation += 1;
        self.first_ack = None;
        self.bytes = 0;
        self.acks = 0;
        self.waiting = false;
    }

    fn success(&mut self, slot: Slot, bytes: u64, now: Instant) {
        if slot.cut == self.cuts {
            self.settle = self.settle.saturating_sub(1);
        }
        if slot.generation != self.generation || (self.limit == self.max && self.previous.is_none())
        {
            return;
        }
        let start = match self.first_ack {
            Some(start) => start,
            None => {
                self.first_ack = Some(now);
                return;
            }
        };
        self.bytes += bytes;
        self.acks += 1;
        let startup = self.cuts == 0;
        let rounds = match startup {
            true => 1,
            false => self.aimd.rounds,
        };
        if self.acks < self.limit.saturating_mul(rounds) || now == start {
            return;
        }
        let rate = self.bytes as f64 / now.duration_since(start).as_secs_f64();
        let measured = self.limit;
        if let Some((limit, baseline)) = self.previous.take() {
            if !self.waiting || rate < baseline * (1.0 + self.aimd.gain) {
                self.limit = limit;
            }
        }
        if self.limit == measured && self.waiting && self.limit < self.max {
            self.previous = (!startup).then_some((self.limit, rate));
            let step = match startup {
                true => self.limit.max(self.max.isqrt().saturating_sub(1)),
                false => (self.limit as f64 * self.aimd.grow).ceil() as usize,
            };
            self.limit = self.limit.saturating_add(step).min(self.max);
        }
        debug!(
            measured,
            bytes_per_sec = rate,
            limit = self.limit,
            "upsert window probe"
        );
        self.reset();
    }

    /// The slot's retry counts toward the window after this response.
    fn throttled(&mut self, slot: &mut Slot) {
        if slot.cut == self.cuts {
            self.settle = self.settle.saturating_sub(1);
            if self.settle == 0 {
                self.limit = ((self.limit as f64 * self.aimd.cut) as usize).max(1);
                self.previous = None;
                self.reset();
                self.cuts += 1;
                self.settle = self.limit;
                debug!(limit = self.limit, cuts = self.cuts, "upsert window cut");
            }
        }
        slot.cut = self.cuts;
    }
}

/// A batch's place in the window, taken when it is sent.
#[derive(Clone, Copy)]
struct Slot {
    generation: u64,
    cut: u64,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

enum Feed {
    Chunk {
        rows: usize,
        cursor: Option<Cursor>,
        last: bool,
    },
    Restarted,
    Skipped {
        chunk: u64,
        row: u64,
        part: Option<String>,
        error: Error,
    },
    Batch {
        partition: Option<String>,
        batch: Batch,
    },
}

enum Fed {
    Read(Result<Option<ReadEvent>, Error>),
    Mapped(Option<Result<Mapped, tokio::task::JoinError>>),
}

/// A chunk's rows as documents with their encoded sizes.
struct Mapped {
    chunk: u64,
    /// The chunk's file and the rows of it before the chunk.
    part: Option<String>,
    first: u64,
    rows: Vec<Result<(Option<String>, u64, Document), Error>>,
}

enum Next {
    Written(Option<Result<BatchOutcome, tokio::task::JoinError>>),
    Feed(Result<Option<Feed>, Error>),
}

#[derive(Default)]
struct BatchOutcome {
    chunks: Vec<(u64, usize)>,
    written: usize,
    bytes: u64,
    skipped: Vec<Error>,
    error: Option<Error>,
}

#[derive(Default)]
struct Batch {
    bytes: u64,
    /// Bytes batched before its first document.
    opened: u64,
    docs: Vec<Document>,
    chunks: Vec<(u64, usize)>,
}

/// The resume point of a run: per file, the rows before its first chunk still in flight.
#[derive(Default)]
struct Positions {
    /// Rows done before this invocation.
    resumed: u64,
    parts: BTreeMap<String, Part>,
    done: BTreeSet<String>,
    /// Rows of the files that left.
    finished: u64,
    /// End of the last file that left.
    tail: Option<Cursor>,
    chunks: HashMap<u64, (String, u64)>,
    next: u64,
}

impl Positions {
    fn push(&mut self, rows: usize, cursor: Option<Cursor>, last: bool) -> u64 {
        let name = cursor.as_ref().map_or("", Cursor::part).to_string();
        let part = self.parts.entry(name.clone()).or_default();
        if let (
            None,
            Some(Cursor::Offset {
                part: file,
                rows: end,
            }),
        ) = (&part.start, &cursor)
        {
            part.start = end.checked_sub(rows as u64).map(|rows| Cursor::Offset {
                part: file.clone(),
                rows,
            });
        }
        part.pushed += rows as u64;
        part.read |= last;
        let consumed = part.pushed;
        let chunk = part
            .checkpoints
            .push(rows, cursor.map(|cursor| Checkpoint { cursor, consumed }));
        let id = self.next;
        self.next += 1;
        self.chunks.insert(id, (name, chunk));
        id
    }

    fn complete(&mut self, id: u64, rows: usize) -> Result<(), Error> {
        let unknown = || Error::InvalidArgument("unknown source chunk".into());
        let (name, chunk) = self.chunks.get(&id).cloned().ok_or_else(unknown)?;
        let part = self.parts.get_mut(&name).ok_or_else(unknown)?;
        if part.checkpoints.complete(chunk, rows)? {
            self.chunks.remove(&id);
        }
        Ok(())
    }

    fn restart(&mut self) {
        self.resumed = 0;
        self.finished = 0;
        for part in self.parts.values_mut() {
            part.pushed = 0;
        }
    }

    /// The resume point, once a chunk or a file finished since the last call. A file leaves
    /// when it is fully read and every chunk of it is done.
    fn ready(&mut self) -> Option<Resume> {
        let mut moved = false;
        for part in self.parts.values_mut() {
            if let Some(mark) = part.checkpoints.ready() {
                part.mark = Some(mark);
                moved = true;
            }
        }
        let left: Vec<String> = self
            .parts
            .iter()
            .filter(|(_, part)| part.read && part.checkpoints.chunks.is_empty())
            .map(|(name, _)| name.clone())
            .collect();
        for name in left {
            if let Some(part) = self.parts.remove(&name) {
                self.finished += part.pushed;
                if self
                    .tail
                    .as_ref()
                    .is_none_or(|tail| tail.part() < name.as_str())
                {
                    self.tail = part.mark.map(|mark| mark.cursor);
                }
                self.done.insert(name);
                moved = true;
            }
        }
        if !moved {
            return None;
        }
        match self.parts.keys().next() {
            Some(first) => self.done.retain(|name| name > first),
            None => self.done.clear(),
        }
        let open: Vec<Cursor> = match self.parts.is_empty() {
            true => self.tail.iter().cloned().collect(),
            false => self
                .parts
                .values()
                .filter_map(|part| match &part.mark {
                    Some(mark) => Some(mark.cursor.clone()),
                    None => part.start.clone(),
                })
                .collect(),
        };
        let rows = self.finished
            + self
                .parts
                .values()
                .filter_map(|part| part.mark.as_ref())
                .map(|mark| mark.consumed)
                .sum::<u64>();
        // An opened file with no row done is not progress.
        if open.is_empty() || rows == 0 {
            return None;
        }
        Some(Resume {
            open,
            done: self.done.iter().cloned().collect(),
            consumed: self.resumed + rows,
        })
    }
}

#[derive(Default)]
struct Part {
    checkpoints: Checkpoints,
    /// Rows read in this invocation.
    pushed: u64,
    /// Where this invocation began reading.
    start: Option<Cursor>,
    mark: Option<Checkpoint>,
    read: bool,
}

/// A chunk's end and the rows of its file read up to it in this invocation.
#[derive(Clone)]
struct Checkpoint {
    cursor: Cursor,
    consumed: u64,
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

    /// True once every row of the chunk is done.
    fn complete(&mut self, id: u64, rows: usize) -> Result<bool, Error> {
        let chunk = self
            .chunks
            .get_mut(&id)
            .ok_or_else(|| Error::InvalidArgument("unknown source chunk".into()))?;
        chunk.0 = chunk
            .0
            .checked_sub(rows)
            .ok_or_else(|| Error::InvalidArgument("source chunk completed twice".into()))?;
        if chunk.0 != 0 {
            return Ok(false);
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
        Ok(true)
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

    use futures::{stream, StreamExt};
    use rstest::rstest;
    use topk_rs::proto::v1::control::FieldSpec;
    use topk_rs::ClientConfig;

    use super::*;

    const AIMD: Aimd = Aimd {
        start: 1,
        max: 64,
        cut: 0.9,
        grow: 0.1,
        gain: 0.05,
        rounds: 3,
    };

    /// Responses of `bytes` each, 100 ms apart, until the window is judged.
    fn judged(window: &mut Window, now: &mut Instant, bytes: u64) {
        window.backlogged();
        let generation = window.generation;
        while window.generation == generation {
            *now += Duration::from_millis(100);
            window.success(window.slot(), bytes, *now);
        }
    }

    #[rstest]
    #[case::one_window_in_startup(0, 5)]
    #[case::rounds_after_a_cut(1, 13)]
    fn responses_before_a_judgement(#[case] cuts: u64, #[case] responses: usize) {
        let mut window = Window::new(64, AIMD);
        (window.limit, window.cuts) = (4, cuts);
        window.backlogged();
        let mut now = Instant::now();
        for _ in 1..responses {
            now += Duration::from_millis(100);
            window.success(window.slot(), 100, now);
            assert_eq!(window.generation, 0);
        }
        now += Duration::from_millis(100);
        window.success(window.slot(), 100, now);
        assert_eq!(window.generation, 1);
    }

    #[test]
    fn opens_at_start_within_max() {
        let start = |start, max| Window::new(max, Aimd { start, ..AIMD }).limit;
        assert_eq!((start(8, 64), start(8, 2)), (8, 2));
    }

    #[rstest]
    #[case::flat_rate(100, 2)]
    #[case::below_gain(103, 2)]
    #[case::throughput_grew(130, 4)]
    #[case::throughput_fell(90, 2)]
    fn throughput_probe(#[case] bytes: u64, #[case] expected: usize) {
        let mut window = Window::new(8, AIMD);
        (window.limit, window.cuts) = (2, 1);
        let mut now = Instant::now();
        for size in [100, bytes] {
            judged(&mut window, &mut now, size);
        }
        assert_eq!(window.limit, expected);
    }

    #[test]
    fn startup_doubles_until_throttled() {
        let mut window = Window::new(128, AIMD);
        let mut now = Instant::now();
        for (expected, bytes) in [(11, 100), (22, 90), (44, 90)] {
            judged(&mut window, &mut now, bytes);
            assert_eq!(window.limit, expected);
        }
        window.throttled(&mut window.slot());
        assert_eq!(window.limit, 39);
        for (expected, bytes) in [(43, 100), (39, 103)] {
            judged(&mut window, &mut now, bytes);
            assert_eq!(window.limit, expected);
        }
    }

    #[test]
    fn throttle_invalidates_probe() {
        let mut window = Window::new(64, AIMD);
        window.limit = 20;
        let mut first = window.slot();
        let mut stale = first;
        window.previous = Some((10, 1.0));
        window.throttled(&mut first);
        window.throttled(&mut stale);
        assert_eq!(window.limit, 18);
        assert!(window.previous.is_none());
        for _ in 0..18 {
            window.success(window.slot(), 100, Instant::now());
        }
        window.throttled(&mut first);
        assert_eq!(window.limit, 16);
        assert_eq!(window.acks, 0);
    }

    #[test]
    fn source_limited_window_stays_put() {
        let mut window = Window::new(8, AIMD);
        for _ in 0..10 {
            window.success(window.slot(), 100, Instant::now());
        }
        assert_eq!(window.limit, 1);
        window.throttled(&mut window.slot());
        assert!(window.admits(0));
    }

    fn test_plan() -> crate::import::plan::Plan {
        crate::import::plan::Plan {
            collection: "c".into(),
            target: crate::import::plan::Target {
                from: crate::import::source::SourceLocation::Object("unused".into()),
                id: "_id".into(),
                filter: None,
                partition: None,
                limit: None,
                exclude: Vec::new(),
                fields: [(
                    "title".into(),
                    crate::import::plan::Field::new(FieldSpec::text(false)),
                )]
                .into(),
            },
        }
    }

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

    fn offset(part: &str, rows: u64) -> Option<Cursor> {
        Some(Cursor::Offset {
            part: part.into(),
            rows,
        })
    }

    #[test]
    fn files_resume_independently() {
        let mut positions = Positions {
            resumed: 10,
            ..Default::default()
        };
        let a1 = positions.push(2, offset("a", 2), false);
        let a2 = positions.push(2, offset("a", 4), false);
        let b1 = positions.push(3, offset("b", 3), false);
        let b_end = positions.push(0, offset("b", 3), true);
        positions.complete(b_end, 0).unwrap();
        assert!(positions.ready().is_none());

        positions.complete(b1, 3).unwrap();
        let resume = positions.ready().unwrap();
        assert_eq!(resume.open, [offset("a", 0).unwrap()]);
        assert_eq!(
            (resume.done.as_slice(), resume.consumed),
            (&["b".to_string()][..], 13)
        );

        positions.complete(a2, 2).unwrap();
        assert!(positions.ready().is_none());
        positions.complete(a1, 2).unwrap();
        let resume = positions.ready().unwrap();
        assert_eq!(resume.open, [offset("a", 4).unwrap()]);
        assert_eq!(
            (resume.done.as_slice(), resume.consumed),
            (&["b".to_string()][..], 17)
        );

        let a_end = positions.push(0, offset("a", 4), true);
        positions.complete(a_end, 0).unwrap();
        let resume = positions.ready().unwrap();
        assert_eq!(resume.open, [offset("b", 3).unwrap()]);
        assert!(resume.done.is_empty());
        assert_eq!(resume.consumed, 17);
        assert!(positions.chunks.is_empty());
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
    async fn rare_partition_flushes_within_window() {
        let mut target = test_plan().target;
        target.partition = Some("tenant".into());
        let rows = std::iter::once("rare")
            .chain(std::iter::repeat_n("hot", 50))
            .enumerate()
            .map(|(id, tenant)| {
                Ok(vec![
                    (
                        "_id".into(),
                        topk_rs::proto::v1::data::Value::string(id.to_string()),
                    ),
                    ("tenant".into(), tenant.into()),
                    ("title".into(), "x".repeat(100).as_str().into()),
                ])
            })
            .collect();
        let chunks = stream::iter([Ok(ReadEvent::Chunk(crate::import::source::Chunk {
            rows,
            cursor: None,
            last: false,
        }))])
        .boxed();
        let options = WriteOptions {
            continue_on_error: false,
            concurrency: Some(1),
            batch_bytes: 1024,
        };
        let partitions: Vec<_> = Import::feed(chunks, target, &options, 1, 0, 1)
            .try_filter_map(|message| async move {
                Ok(match message {
                    Feed::Batch { partition, .. } => partition,
                    _ => None,
                })
            })
            .try_collect()
            .await
            .unwrap();
        let rare = partitions.iter().position(|p| p == "rare").unwrap();
        assert!(rare < partitions.len() - 1, "{partitions:?}");
    }

    #[tokio::test]
    async fn cancelled_writes_hold_lock() {
        let dir = tempfile::tempdir().unwrap();
        let plan = test_plan();
        let run = Run::start(
            dir.path(),
            crate::import::state::State::new(
                String::new(),
                crate::scope::Scope {
                    project_id: "p1".parse().unwrap(),
                    region: "unused".into(),
                },
                plan,
                ReadOptions { max_memory: None },
                WriteOptions {
                    continue_on_error: false,
                    concurrency: Some(1),
                    batch_bytes: 1,
                },
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
            last: false,
        }))])
        .boxed();
        let import = Import {
            run,
            client: Client::new(ClientConfig::new("unused", "unused")),
            scan: chunks,
            mappers: 1,
            create: false,
        };
        let mut execution = Box::pin(import.execute(AIMD));
        assert!(matches!(
            execution.try_next().await.unwrap(),
            Some(Event::Progress { written: 0, .. })
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(200), execution.try_next())
                .await
                .is_err()
        );
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
        let plan = test_plan();
        let run = Run::start(
            dir.path(),
            crate::import::state::State::new(
                String::new(),
                crate::scope::Scope {
                    project_id: "p1".parse().unwrap(),
                    region: "unused".into(),
                },
                plan,
                ReadOptions { max_memory: None },
                WriteOptions {
                    continue_on_error: true,
                    concurrency: Some(1),
                    batch_bytes: 1024,
                },
            ),
        )
        .await
        .unwrap();
        let id = run.id;
        let chunks = async_stream::try_stream! {
            yield ReadEvent::Chunk(crate::import::source::Chunk {
                rows: vec![Err(Error::CannotCoerce("int"))],
                cursor: Some(crate::import::source::Cursor::Key("1".into())),
                last: false,
            });
            if stream_error {
                Err(Error::InvalidArgument("source stream failed".into()))?;
            }
        }
        .boxed();
        let import = Import {
            run,
            client: Client::new(ClientConfig::new("unused", "unused")),
            scan: chunks,
            mappers: 1,
            create: false,
        };
        let mut execution = Box::pin(import.execute(AIMD));
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
            assert!(matches!(&run.state().progress, Some(Progress::After(c)) if c.consumed == 1));
        } else {
            assert!(
                matches!(execution.try_next().await.unwrap(), Some(Event::Completed { outcome }) if outcome.rows == 0 && outcome.failed == 1)
            );
            assert!(execution.try_next().await.unwrap().is_none());
            assert!(!dir.path().join(format!("{id}.toml")).exists());
            assert!(!dir.path().join(format!("{id}.lock")).exists());
        }
    }
}
