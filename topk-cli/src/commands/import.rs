use std::collections::BTreeMap;
use std::io::{ErrorKind, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::Args;
use futures::{StreamExt, TryStreamExt};
use indicatif::{MultiProgress, ProgressDrawTarget};
use tokio::sync::Semaphore;

use crate::endpoint::Endpoint;
use crate::import::{
    self, render, Cursor, Error, LoadOutcome, Scan, Sink, Source, Spec, State, Uri, ID,
    ID_PLACEHOLDER,
};

const OBJECT_CONCURRENCY: usize = 8;

#[derive(Args, Debug)]
// Clap's generated usage renders `<SOURCE>` as required; it is not, with --spec.
#[command(override_usage = "topk import [OPTIONS] [SOURCE] [OBJECTS]...")]
pub struct ImportArgs {
    #[arg(
        required_unless_present_any = ["spec", "resume"],
        help = "Source URL, file path, or glob; also required with --spec or --resume, except for files"
    )]
    pub source: Option<Uri>,
    #[arg(
        conflicts_with = "spec",
        help = "Source objects to import; exact names, globs, or <object>=<collection>"
    )]
    pub objects: Vec<String>,

    #[arg(
        short = 'f',
        long,
        value_name = "SPEC",
        help = "Read a TOML import spec"
    )]
    pub spec: Option<PathBuf>,
    #[arg(
        long,
        value_name = "RUN",
        conflicts_with_all = ["objects", "to", "id", "filter", "limit"],
        help = "Continue a run that stopped, by the id in its header; -f swaps in an edited spec"
    )]
    pub resume: Option<String>,
    #[arg(
        long,
        help = "Print the spec and a sample of documents, without importing"
    )]
    pub dry_run: bool,

    #[arg(
        long,
        value_name = "COLLECTION",
        conflicts_with = "spec",
        help = "Name the target collection; required when it cannot be derived"
    )]
    pub to: Option<String>,
    #[arg(
        long,
        value_name = "COLUMN",
        conflicts_with = "spec",
        help = "Column to use as the document id (_id); use when it can't be auto-detected"
    )]
    pub id: Option<String>,
    #[arg(long, help = "Import into this partition")]
    pub partition: Option<String>,
    #[arg(
        conflicts_with = "spec",
        long,
        help = "Only read rows matching a filter, in the source's language: \
                SQL WHERE for files and databases, a JSON query for mongodb/elasticsearch"
    )]
    pub filter: Option<String>,
    // Broadcasts to every collection like --partition, so it is allowed with a spec.
    #[arg(long, help = "Read at most this many rows per object")]
    pub limit: Option<u64>,

    #[arg(short = 'y', long, help = "Skip confirmation")]
    pub yes: bool,
    #[arg(
        long,
        help = "Skip documents that fail instead of stopping; exits non-zero if any did"
    )]
    pub continue_on_error: bool,
    #[arg(
        short = 'c',
        long,
        default_value = "16",
        value_parser = clap::value_parser!(u32).range(1..=256),
        help = "Concurrent upserts in flight, budgeted across the whole run"
    )]
    pub concurrency: u32,
    #[arg(
        long,
        default_value = "8MiB",
        value_name = "SIZE",
        help = "Bytes of documents per upsert"
    )]
    pub batch_bytes: bytesize::ByteSize,
}

async fn plan(source: &Source, args: &ImportArgs, given: Option<Spec>) -> Result<Spec, Error> {
    let mut spec: Spec = match given {
        Some(spec) => spec,
        None => {
            let discovered = import::discover(
                source,
                &args.objects,
                args.to.as_deref(),
                args.id.as_deref(),
            )
            .await?;
            for skipped in discovered.skipped {
                eprintln!("{skipped}");
            }
            discovered.spec
        }
    };
    // A filter names one object's columns.
    if args.filter.is_some() && spec.collections.len() > 1 {
        return Err(Error::InvalidArgument(format!(
            "--filter applies to a single object, but {} objects matched — \
             set `filter` per collection in a spec",
            spec.collections.len()
        )));
    }
    for target in spec.collections.values_mut() {
        if let Some(filter) = &args.filter {
            target.filter = Some(filter.clone());
        }
        if let Some(limit) = args.limit {
            target.limit = Some(limit);
        }
        if let Some(partition) = &args.partition {
            target.partition = Some(partition.clone());
        }
    }
    Ok(spec)
}

fn confirm(collections: usize, region: &str) -> Result<bool, Error> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::InvalidArgument(
            "not a terminal: pass --yes to import without confirmation".to_string(),
        ));
    }
    let s = if collections == 1 { "" } else { "s" };
    match dialoguer::Confirm::new()
        .with_prompt(format!("Import {collections} collection{s} into {region}?"))
        .default(false)
        .interact()
    {
        Ok(yes) => Ok(yes),
        Err(dialoguer::Error::IO(e)) if e.kind() == ErrorKind::Interrupted => Ok(false),
        Err(dialoguer::Error::IO(e)) => Err(e.into()),
    }
}

fn human(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    match (secs / 3600, secs % 3600 / 60, secs % 60) {
        (0, 0, s) => format!("{s}s"),
        (0, m, s) => format!("{m}m{s:02}s"),
        (h, m, s) => format!("{h}h{m:02}m{s:02}s"),
    }
}

fn report(outcomes: &BTreeMap<String, LoadOutcome>, json: bool) -> Result<ExitCode, Error> {
    if json {
        println!("{}", serde_json::to_string(outcomes)?);
    } else {
        for (name, outcome) in outcomes {
            let took = human(outcome.elapsed);
            match outcome.failed {
                0 => eprintln!("{name}: {} rows written in {took}", outcome.rows),
                failed => eprintln!(
                    "{name}: {} rows written, {failed} failed in {took}",
                    outcome.rows
                ),
            }
        }
    }
    Ok(match outcomes.values().all(|o| o.failed == 0) {
        true => ExitCode::SUCCESS,
        false => ExitCode::FAILURE,
    })
}

async fn execute(
    sink: &Sink<'_>,
    scans: &[Scan],
    state: State,
    readers: usize,
) -> Result<BTreeMap<String, LoadOutcome>, Error> {
    // An unwritable config dir costs the ability to resume, not the import.
    if let Err(e) = state.save() {
        eprintln!("cannot save run state ({e}) — this run cannot be resumed");
    }
    let state = Mutex::new(state);
    let checkpoint = |name: &str, cursor: Cursor| {
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        state.cursors.insert(name.to_string(), cursor);
        // A lost checkpoint costs a redo, never a skip.
        if let Err(e) = state.save() {
            tracing::warn!(%e, "cannot save run state");
        }
    };
    futures::stream::iter(scans.iter())
        .map(|scan| {
            let checkpoint = &checkpoint;
            // A resumed limit would be applied again from the cursor, so a
            // limited collection is never checkpointed: it restarts whole.
            let checkpoint_cursor = move |cursor: &str| {
                if scan.target.limit.is_none() {
                    checkpoint(&scan.name, Cursor::After(cursor.to_string()));
                }
            };
            async move {
                let outcome = sink.load(scan, checkpoint_cursor).await?;
                checkpoint(&scan.name, Cursor::Done);
                Ok((scan.name.clone(), outcome))
            }
        })
        .buffer_unordered(readers)
        .try_collect()
        .await
}

pub async fn run(endpoint: &Endpoint, args: &ImportArgs, json: bool) -> anyhow::Result<ExitCode> {
    tracing::info!(?args, "import");
    let resumed = args.resume.as_deref().map(State::load).transpose()?;
    // Credentials never enter a spec: the CLI uri is the source, or every `from`
    // is its own file locator.
    let uri = args.source.clone().unwrap_or_default();
    let given: Option<Spec> = match (&args.spec, &resumed) {
        (Some(path), _) => {
            let text = std::fs::read_to_string(path).map_err(|e| {
                Error::InvalidArgument(format!("cannot read spec {}: {e}", path.display()))
            })?;
            Some(toml::from_str(&text)?)
        }
        (None, Some(state)) => Some(toml::from_str(&state.spec)?),
        (None, None) => None,
    };
    let source = Source::connect(&uri, endpoint).await?;
    let mut spec = plan(&source, args, given).await?;

    let source_name = uri.to_string();
    // Stored for --resume, and compared per collection against an edited -f.
    let stored = toml::to_string_pretty(&spec)
        .map_err(|e| Error::InvalidArgument(format!("cannot serialize spec: {e}")))?;
    let run = args.resume.clone().unwrap_or_else(State::id);
    let mut state =
        resumed.unwrap_or_else(|| State::new(run.clone(), source_name.clone(), stored.clone()));
    let (done, after) = state.reconcile(&source_name, &mut spec, stored)?;
    if spec.collections.is_empty() {
        eprintln!("run {run}: all {done} collection(s) already imported");
        State::remove(&run);
        return Ok(ExitCode::SUCCESS);
    }
    if args.dry_run {
        print!("{}", render(&spec, None, &after));
        for (name, target) in spec.collections.iter() {
            if target.id.as_deref() == Some(ID_PLACEHOLDER) {
                eprintln!("# {name}: set `id` above (or pass --id) to preview rows");
                continue;
            }
            import::preview(name, &source, target).await?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    for (name, target) in spec.collections.iter() {
        if target.id.as_deref() == Some(ID_PLACEHOLDER) {
            return Err(Error::InvalidArgument(format!(
                "{name}: couldn't detect an id column — pass `--id <column>`, \
                 or set `id` in a spec (it becomes each document's `{ID}`)"
            ))
            .into());
        }
    }
    let scans = spec
        .collections
        .iter()
        .map(|(name, target)| source.scan(name, target, after.get(name).map(String::as_str)))
        .collect::<Result<Vec<_>, Error>>()?;
    let client = endpoint.client()?;
    let mut pending = import::absent(&client, &spec).await?;
    // `--limit 0` reads nothing, so it must not leave an empty collection behind
    // for the next run's schema to collide with.
    pending.retain(|name, _| spec.collections.get(name).and_then(|t| t.limit) != Some(0));
    let fresh: Vec<&str> = pending.keys().map(String::as_str).collect();
    // Before the run: a killed run prints nothing after.
    eprintln!(
        "# run {run}{}",
        match done {
            0 => String::new(),
            n => format!(", resuming: {n} collection(s) done"),
        }
    );
    eprint!("{}", render(&spec, Some(&fresh), &after));
    let region = endpoint.region.as_deref().unwrap_or_default();
    if !args.yes && !confirm(spec.collections.len(), region)? {
        return Ok(ExitCode::SUCCESS);
    }

    let progress = match json {
        false => MultiProgress::new(),
        true => MultiProgress::with_draw_target(ProgressDrawTarget::hidden()),
    };
    if !json {
        import::set_progress(progress.clone());
    }
    let sink = Sink {
        client: &client,
        progress: &progress,
        budget: Arc::new(Semaphore::new(args.concurrency as usize)),
        batch_bytes: args.batch_bytes.as_u64() as usize,
        continue_on_error: args.continue_on_error,
    };
    for (name, schema) in pending {
        import::create(&client, &name, schema).await?;
    }
    let readers = scans
        .len()
        .min(OBJECT_CONCURRENCY)
        .min(source.concurrency_limit())
        .max(1);
    let resume_hint = || {
        eprintln!(
            "nothing else was imported; to continue: topk import {}--resume {run}",
            match args.source.is_none() {
                true => String::new(),
                false => format!("'{source_name}' "),
            }
        )
    };
    let outcomes = tokio::select! {
        outcomes = execute(&sink, &scans, state, readers) => outcomes,
        _ = tokio::signal::ctrl_c() => {
            let _ = progress.clear();
            resume_hint();
            return Ok(ExitCode::from(130));
        }
    };
    let outcomes = outcomes.inspect_err(|_| resume_hint())?;
    State::remove(&run);
    Ok(report(&outcomes, json)?)
}
