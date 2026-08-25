use std::collections::BTreeMap;
use std::io::{IsTerminal, Write as _};
use std::path::PathBuf;

use clap::Args;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use topk_rs::Client;

use crate::import::{
    self, Cursor, Duckdb, Error, Outcome, Source, Spec, State, Target, Uri, ID, ID_PLACEHOLDER,
};
use crate::output::{Output, OutputFormat};

const CONCURRENCY: usize = 8;
const PREVIEW_ROWS: usize = 5;
const PREVIEW_ELEMENTS: usize = 8;
const PREVIEW_CHARS: usize = 120;
const PREVIEW_BYTES: usize = 8;

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

pub async fn run(
    client: impl FnOnce() -> Result<Client, Error>,
    args: &ImportArgs,
    output: &Output,
) -> Result<(), Error> {
    tracing::info!(?args, "import");
    let resumed = args.resume.as_deref().map(State::load).transpose()?;
    // Credentials never enter a spec: the CLI uri is the source, or every `from`
    // is its own file locator.
    let source = match args.source.as_ref() {
        Some(uri) => import::connect(uri).await?,
        None => Source::Duck(Duckdb::Files(None)),
    };
    let mut spec = match (&args.spec, &resumed) {
        (Some(path), _) => {
            if !args.objects.is_empty() {
                return Err(Error::InvalidArgument(
                    "a spec already names its objects; drop them from the command line".to_string(),
                ));
            }
            let text = std::fs::read_to_string(path).map_err(|e| {
                Error::InvalidArgument(format!("cannot read spec {}: {e}", path.display()))
            })?;
            Spec::parse(&text)?
        }
        (None, Some(state)) => Spec::parse(&state.spec)?,
        (None, None) => {
            let discovered = import::discover(
                &source,
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

    // A dry run may print the `<column>` placeholder for the user to fill in.
    if !args.dry_run {
        for (name, target) in spec.collections.iter() {
            if target.id.as_deref() == Some(ID_PLACEHOLDER) {
                return Err(Error::InvalidArgument(format!(
                    "{name}: couldn't detect an id column — pass `--id <column>`, \
                     or set `id` in a spec (it becomes each document's `{ID}`)"
                )));
            }
        }
    }

    // Credentials never enter a spec: the CLI uri is the source, or every `from` is a file.
    let source_name = args.source.as_ref().map(Uri::redacted).unwrap_or_default();
    // Fail before any prompt, not on the first read.
    if args.source.is_none() {
        for target in spec.collections.values() {
            Duckdb::file(&target.from)?;
        }
    }

    // Stored for --resume, and compared per collection against an edited -f.
    let plan = toml::to_string_pretty(&spec)
        .map_err(|e| Error::InvalidArgument(format!("cannot serialize spec: {e}")))?;
    let run = args.resume.clone().unwrap_or_else(State::id);
    let mut done: Vec<String> = Vec::new();
    let mut after: BTreeMap<String, String> = BTreeMap::new();
    let state = match resumed {
        Some(mut state) => {
            if state.source != source_name {
                return Err(Error::InvalidArgument(format!(
                    "run {run} reads {}, not {source_name:?}",
                    match state.source.is_empty() {
                        true => "files".to_string(),
                        false => format!("{:?}", state.source),
                    }
                )));
            }
            // A cursor only holds for an unchanged target.
            let stored = Spec::parse(&state.spec)?;
            state.cursors.retain(|name, cursor| {
                let (Some(target), Some(was)) =
                    (spec.collections.get(name), stored.collections.get(name))
                else {
                    return false;
                };
                if toml::to_string(target).ok() != toml::to_string(was).ok() {
                    eprintln!("# {name}: spec changed, starting over");
                    return false;
                }
                match cursor {
                    Cursor::Done => done.push(name.clone()),
                    Cursor::After(mark) => {
                        after.insert(name.clone(), mark.clone());
                    }
                }
                true
            });
            spec.collections.retain(|name, _| !done.contains(name));
            state.spec = plan;
            state
        }
        None => State::new(source_name.clone(), plan),
    };
    if spec.collections.is_empty() {
        output.meta(&format!(
            "run {run}: all {} collection(s) already imported",
            done.len()
        ));
        State::remove(&run);
        return Ok(());
    }

    if args.dry_run {
        print!("{}", render(&spec, None, &after));
        for (name, target) in spec.collections.iter() {
            if target.id.as_deref() == Some(ID_PLACEHOLDER) {
                eprintln!("# {name}: set `id` above (or pass --id) to preview rows");
                continue;
            }
            preview(name, &source, target).await?;
        }
        return Ok(());
    }

    let client = client()?;
    let mut pending = import::absent(&client, &spec).await?;
    let fresh: Vec<String> = pending.keys().cloned().collect();
    // Before the run: a killed run prints nothing after.
    eprintln!(
        "# run {run}{}",
        match done.len() {
            0 => String::new(),
            n => format!(", resuming: {n} collection(s) done"),
        }
    );
    eprint!("{}", render(&spec, Some(&fresh), &after));
    if !args.yes {
        if !std::io::stdin().is_terminal() {
            return Err(Error::InvalidArgument(
                "not a terminal: pass --yes to import without confirmation".to_string(),
            ));
        }
        eprint!("Import {} collection(s)? [y/N] ", spec.collections.len());
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes") {
            output.meta("Aborted.");
            return Ok(());
        }
    }
    // An unwritable config dir costs the ability to resume, not the import.
    if let Err(e) = state.save(&run) {
        eprintln!("cannot save run state ({e}) — this run cannot be resumed");
    }
    let state = std::sync::Mutex::new(state);
    let checkpoint = |name: &str, cursor: Cursor| {
        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
        state.cursors.insert(name.to_string(), cursor);
        // A lost checkpoint costs a redo, never a skip.
        if let Err(e) = state.save(&run) {
            tracing::warn!(%e, "cannot save run state");
        }
    };

    // Each duckdb scan holds a row group; too many OOM-kill the process
    // regardless of the per-connection `memory_limit`.
    let readers = match &source {
        Source::Duck(Duckdb::Files(_)) => {
            import::max_readers().unwrap_or(CONCURRENCY) / (1 + import::READ_AHEAD)
        }
        Source::Duck(_) => import::max_readers().unwrap_or(CONCURRENCY),
        _ => CONCURRENCY,
    };
    // `-c` is the run's budget: all collections share one HTTP/2 connection
    // whose window is sized from it.
    let objects = spec
        .collections
        .len()
        .min(CONCURRENCY)
        .min(readers)
        .min(args.concurrency as usize)
        .max(1);
    let upserts = (args.concurrency as usize / objects).max(1);
    let batch_bytes = args.batch_bytes.as_u64() as usize;
    let progress = output.spinner("importing");
    // Concurrent collections share the spinner.
    let total_rows = std::sync::atomic::AtomicUsize::new(0);
    let count = spec.collections.len();
    let outcomes: Result<Vec<_>, Error> = futures::stream::iter(spec.collections.iter())
        .map(|(name, target)| {
            let schema = pending.remove(name.as_str());
            let (progress, total_rows, checkpoint) = (&progress, &total_rows, &checkpoint);
            import::load(
                &client,
                name,
                &source,
                target,
                schema,
                after.get(name.as_str()).map(String::as_str),
                args.continue_on_error,
                batch_bytes,
                upserts,
                move |n| {
                    let total = total_rows.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    progress.set_message(match count {
                        1 => format!("{name}: {n} rows"),
                        _ => format!("{count} collections: {total} rows"),
                    })
                },
                move |mark| checkpoint(name, Cursor::After(mark.to_string())),
            )
            .map_ok(move |outcome| {
                checkpoint(name, Cursor::Done);
                (name, outcome)
            })
        })
        .buffer_unordered(objects)
        .try_collect()
        .await;
    progress.finish();
    let outcomes = match outcomes {
        Ok(outcomes) => outcomes,
        Err(e) => {
            eprintln!(
                "to continue: topk import {}--resume {run}",
                match source_name.is_empty() {
                    true => String::new(),
                    false => format!("'{source_name}' "),
                }
            );
            return Err(match e.skippable() {
                true => Error::InvalidArgument(format!(
                    "{e}\nnothing else was imported — fix the source and resume"
                )),
                false => e,
            });
        }
    };
    State::remove(&run);

    if matches!(output.format, OutputFormat::Json) {
        let json: BTreeMap<_, _> = outcomes
            .iter()
            .map(|(name, outcome)| (name, outcome))
            .collect();
        output.print_json(&json)?;
    } else {
        for (name, outcome) in &outcomes {
            let mut line = summary(outcome);
            // The plan said "will create"; if nothing was read, we didn't.
            if outcome.rows == 0 && fresh.iter().any(|f| f == *name) {
                line.push_str(" — collection not created");
            }
            output.meta(&format!("{name}: {line}"));
        }
    }

    if outcomes.iter().any(|(_, outcome)| outcome.failed > 0) {
        return Err(Error::Skipped(
            outcomes.iter().map(|(_, o)| o.failed).sum::<usize>(),
        ));
    }
    Ok(())
}

fn summary(outcome: &Outcome) -> String {
    let mut line = match outcome.failed {
        0 => format!("{} rows written", outcome.rows),
        failed => format!("{} rows written, {failed} failed", outcome.rows),
    };
    if outcome.bytes > 0 && outcome.elapsed_ms > 0 {
        let rate = outcome.bytes as f64 * 1000.0 / outcome.elapsed_ms as f64;
        line.push_str(&format!(
            " — {} in {:.1}s, {}/s, upsert p50 {}ms p99 {}ms",
            bytesize::ByteSize(outcome.bytes as u64).to_string_as(true),
            outcome.elapsed_ms as f64 / 1000.0,
            bytesize::ByteSize(rate as u64).to_string_as(true),
            outcome.upsert_p50_ms,
            outcome.upsert_p99_ms,
        ));
    }
    line
}

/// What prints here is what `--spec` would re-run. `fresh` is None before a
/// cluster has been consulted (--dry-run); `after` holds the resume cursors.
fn render(spec: &Spec, fresh: Option<&[String]>, after: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut indexed: Vec<String> = Vec::new();
    // An unindexed float_list imports fine and silently is not searchable.
    let lists = spec
        .collections
        .values()
        .flat_map(|target| target.fields.values())
        .any(|field| matches!(field.ty, import::Type::FloatList) && field.index.is_none());
    for (name, target) in spec.collections.iter() {
        if let Some(fresh) = fresh {
            let state = match fresh.iter().any(|created| created == name) {
                true => "will create",
                false => "exists",
            };
            out.push_str(&format!("# {state}\n"));
        }
        if let Some(mark) = after.get(name) {
            out.push_str(&format!(
                "# resuming after {}\n",
                elide(&serde_json::Value::String(mark.clone()))
            ));
        }
        out.push_str(&format!("[{}]\n", key(name)));
        for (field, value) in [
            ("from", Some(target.from.as_str())),
            ("id", target.id.as_deref()),
            ("filter", target.filter.as_deref()),
            ("partition", target.partition.as_deref()),
        ] {
            if let Some(value) = value {
                out.push_str(&format!("{field} = {}\n", string(value)));
            }
        }
        if let Some(limit) = target.limit {
            out.push_str(&format!("limit = {limit}\n"));
        }
        out.push_str(&format!("\n[{}.fields]\n", key(name)));
        for (field, spec) in target.fields.iter() {
            let inline = toml::Value::try_from(spec)
                .map(|value| value.to_string())
                .unwrap_or_default();
            out.push_str(&format!("{} = {inline}\n", key(field)));
            if let Some(index) = &spec.index {
                let index = toml::Value::try_from(index)
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                indexed.push(format!("{field} ({})", index.trim_matches('"')));
            }
        }
        out.push('\n');
    }
    match indexed.is_empty() {
        true => out.push_str(
            "# no indexes declared — the data will import but will not be searchable. Add one:\n\
             #   text    index = \"keyword\" | \"exact\" | \"semantic\" | \"ngram\"\n\
             #   vector  index = { vector = { metric = \"cosine\" } }\n\
             #   matrix  index = { multi_vector = {} }\n",
        ),
        false => out.push_str(&format!("# indexed: {}\n", indexed.join(", "))),
    }
    if lists {
        out.push_str(
            "# a float_list is not searchable as-is; for vector search use: \
             { type = \"f32_vector\", dim = <N>, index = { vector = { metric = \"cosine\" } } }\n",
        );
    }
    out
}

fn key(name: &str) -> String {
    match name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        true => name.to_string(),
        false => string(name),
    }
}

fn string(value: &str) -> String {
    toml::Value::String(value.to_string()).to_string()
}

async fn preview(name: &str, source: &import::Source, target: &Target) -> Result<(), Error> {
    // Cap at the source: a dropped ES stream leaks its point-in-time.
    let cap = PREVIEW_ROWS as u64;
    let target = Target {
        limit: Some(target.limit.map_or(cap, |limit| limit.min(cap))),
        ..target.clone()
    };
    let mut rows = import::documents(source, &target, None).await?;
    let mut shown = 0;
    while let Some(row) = rows.next().await {
        let doc = row?;
        if shown == 0 {
            eprintln!("# → {name}");
        }
        shown += 1;

        let mut pairs = doc
            .fields
            .into_iter()
            .map(|(key, value)| {
                Ok((
                    key,
                    match value.as_binary() {
                        Some(bytes) => serde_json::Value::String(binary(bytes)),
                        None => serde_json::Value::try_from(value)?,
                    },
                ))
            })
            .collect::<Result<Vec<(String, serde_json::Value)>, topk_rs::Error>>()?;
        pairs.sort_by_key(|(k, _)| (k != ID, k.clone()));
        let doc = serde_json::Value::Object(pairs.into_iter().collect());
        // stderr, so `--dry-run > spec.toml` captures the spec alone.
        eprintln!("{}", elide(&doc));
    }
    if shown as u64 == cap {
        eprintln!("# … showing the first {PREVIEW_ROWS} rows");
    }
    Ok(())
}

/// Length, not content: the length is what tells you the `dim` to declare.
fn binary(bytes: &[u8]) -> String {
    let head: Vec<String> = bytes
        .iter()
        .take(PREVIEW_BYTES)
        .map(|b| format!("{b:02x}"))
        .collect();
    let more = match bytes.len() > PREVIEW_BYTES {
        true => " …",
        false => "",
    };
    format!("<{} bytes: {}{more}>", bytes.len(), head.join(" "))
}

fn elide(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Array(items) if items.len() > PREVIEW_ELEMENTS => {
            let head: Vec<String> = items.iter().take(2).map(elide).collect();
            let tail: Vec<String> = items[items.len() - 2..].iter().map(elide).collect();
            format!(
                "[{}, … {} more, {}]",
                head.join(", "),
                items.len() - 4,
                tail.join(", ")
            )
        }
        serde_json::Value::String(text) if text.chars().count() > PREVIEW_CHARS => {
            let head: String = text.chars().take(PREVIEW_CHARS).collect();
            format!("{head:?}…")
        }
        serde_json::Value::Object(entries) => {
            let pairs: Vec<String> = entries
                .iter()
                .map(|(key, value)| format!("{key:?}: {}", elide(value)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        other => other.to_string(),
    }
}
