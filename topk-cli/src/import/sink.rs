use std::collections::{BTreeMap, HashMap, VecDeque};
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

use crate::commands::import::ImportArgs;
use crate::endpoint::Endpoint;
use crate::import::ddl::{self, Schema};
use crate::import::decode::{self, id_string};
use crate::import::error::{Error, MAX_DOC_BYTES};
use crate::import::source::{Cursor, Record, Scan, Source};
use crate::import::spec::{Spec, Target};
use crate::import::state::{Mark, State};
use crate::import::{ID, ID_PLACEHOLDER};

#[derive(Default, serde::Serialize)]
pub struct LoadOutcome {
    /// Rows written; rows sharing an id collapse into one document (upsert).
    pub rows: usize,
    /// Rows skipped by --continue-on-error.
    pub failed: usize,
    #[serde(skip)]
    pub elapsed: Duration,
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

pub struct Import {
    client: Client,
    scans: IndexMap<String, Scan>,
    pub pending: HashMap<String, Schema>,
    readers: usize,
    budget: Arc<Semaphore>,
    batch_bytes: usize,
    continue_on_error: bool,
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

impl Import {
    pub async fn prepare(
        endpoint: &Endpoint,
        source: &Source,
        spec: &Spec,
        after: &BTreeMap<String, Cursor>,
        args: &ImportArgs,
    ) -> Result<Import, Error> {
        for (name, target) in &spec.collections {
            if target.id.as_deref() == Some(ID_PLACEHOLDER) {
                return Err(Error::InvalidArgument(format!(
                    "{name}: couldn't detect an id column — pass `--id <column>`, \
                     or set `id` in a spec (it becomes each document's `{ID}`)"
                )));
            }
        }
        let scans = spec
            .collections
            .iter()
            .map(|(name, target)| {
                Ok((name.clone(), source.scan(target, after.get(name).cloned())?))
            })
            .collect::<Result<IndexMap<_, _>, Error>>()?;
        let client = endpoint
            .client()
            .map_err(|e| Error::InvalidArgument(e.to_string()))?;
        let mut pending = ddl::absent(&client, spec).await?;
        // `--limit 0` reads nothing, so it must not leave an empty collection behind
        // for the next run's schema to collide with.
        pending.retain(|name, _| spec.collections.get(name).and_then(|t| t.limit) != Some(0));
        Ok(Import {
            client,
            readers: scans.len().min(8).min(source.concurrency_limit()).max(1),
            scans,
            pending,
            budget: Arc::new(Semaphore::new(args.concurrency as usize)),
            batch_bytes: args.batch_bytes.as_u64() as usize,
            continue_on_error: args.continue_on_error,
        })
    }

    pub async fn execute(
        mut self,
        state: State,
        progress: &MultiProgress,
    ) -> Result<BTreeMap<String, LoadOutcome>, Error> {
        // An unwritable config dir costs the ability to resume, not the import.
        if let Err(e) = state.save() {
            crate::import::note(format!(
                "cannot save run state ({e}) — this run cannot be resumed"
            ));
        }
        let state = Mutex::new(state);
        for (name, schema) in std::mem::take(&mut self.pending) {
            ddl::create(&self.client, &name, schema).await?;
        }
        let outcomes = stream::iter(std::mem::take(&mut self.scans))
            .map(|(name, scan)| {
                let state = &state;
                let import = &self;
                async move {
                    Ok::<_, Error>((
                        name.clone(),
                        import.load_one(&name, scan, state, progress).await?,
                    ))
                }
            })
            .buffer_unordered(self.readers)
            .try_collect()
            .await?;
        State::remove(&state.into_inner().unwrap_or_else(|e| e.into_inner()).id);
        Ok(outcomes)
    }

    async fn load_one(
        &self,
        name: &str,
        scan: Scan,
        state: &Mutex<State>,
        progress: &MultiProgress,
    ) -> Result<LoadOutcome, Error> {
        let Scan { target, mut chunks } = scan;
        let started = Instant::now();
        let bar = Spinner::add(progress, name);
        let mut writer = BatchWriter {
            import: self,
            state,
            name,
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
            if chunk.cursor.is_some() {
                writer.cursor = chunk.cursor;
            }
        }
        writer.finish().await?;
        Self::checkpoint(state, name, Mark::Done);
        outcome.elapsed = started.elapsed();
        Ok(outcome)
    }

    fn checkpoint(state: &Mutex<State>, name: &str, mark: Mark) {
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
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
    import: &'a Import,
    state: &'a Mutex<State>,
    name: &'a str,
    checkpoint: bool,
    batches: IndexMap<Option<String>, Vec<Document>>,
    /// Buffered across every partition.
    bytes: usize,
    /// Rows arrive before the cursor that covers them; it rides with the next flush.
    cursor: Option<Cursor>,
    /// A cursor is checkpointed once every preceding batch has completed.
    inflight: InflightBatches,
}

impl BatchWriter<'_> {
    async fn push(&mut self, partition: Option<String>, doc: Document) -> Result<(), Error> {
        self.bytes += doc.encoded_len();
        self.batches.entry(partition).or_default().push(doc);
        if self.bytes >= self.import.batch_bytes {
            while let Some((partition, docs)) = self.batches.pop() {
                self.flush(partition, docs).await?;
            }
            self.bytes = 0;
        }
        Ok(())
    }

    async fn flush(&mut self, partition: Option<String>, docs: Vec<Document>) -> Result<(), Error> {
        // Waits while the run is at `-c`; spawned upserts keep landing meanwhile.
        let permit = self
            .import
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
        let mut collection = self.import.client.collection(self.name);
        if let Some(partition) = partition {
            collection = collection.partition(partition);
        }
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
                Import::checkpoint(self.state, self.name, Mark::After(cursor));
            }
        }
        Ok(())
    }

    async fn finish(mut self) -> Result<(), Error> {
        while let Some((partition, batch)) = self.batches.pop() {
            self.flush(partition, batch).await?;
        }
        while !self.inflight.is_empty() {
            self.complete_next().await?;
        }
        Ok(())
    }
}
