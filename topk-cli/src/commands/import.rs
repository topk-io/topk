use std::collections::BTreeMap;
use std::io::{IsTerminal, Write as _};
use std::path::PathBuf;

use clap::Args;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use topk_rs::Client;

use crate::import::{self, Duckdb, Error, Outcome, Source, Spec, Target, Uri, ID, ID_PLACEHOLDER};
use crate::output::{Output, OutputFormat};

const CONCURRENCY: usize = 8;
const PREVIEW_ROWS: usize = 5;
const PREVIEW_ELEMENTS: usize = 8;
const PREVIEW_CHARS: usize = 120;

#[derive(Args)]
// Clap's auto-generated error usage renders `<SOURCE>` as required (it is not,
// with --spec) and drops [OPTIONS].
#[command(override_usage = "topk import [OPTIONS] [SOURCE] [OBJECTS]...")]
pub struct ImportArgs {
    #[arg(
        required_unless_present = "spec",
        help = "Source URL, file path, or glob; also required with --spec, except for files"
    )]
    pub source: Option<Uri>,
    #[arg(
        conflicts_with = "spec",
        help = "Source objects to import; exact names, globs, or <object>=<collection>"
    )]
    pub objects: Vec<String>,

    #[arg(short = 'f', long, value_name = "SPEC", help = "Read a TOML import spec")]
    pub spec: Option<PathBuf>,
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
    #[arg(conflicts_with = "spec", long, help = "Read at most this many rows")]
    pub limit: Option<u64>,

    #[arg(short = 'y', long, help = "Skip confirmation")]
    pub yes: bool,
    #[arg(
        long,
        help = "Skip documents that fail instead of stopping; exits non-zero if any did"
    )]
    pub continue_on_error: bool,
}

pub async fn run(
    client: impl FnOnce() -> Result<Client, Error>,
    args: &ImportArgs,
    output: &Output,
) -> Result<(), Error> {
    let mut spec = match &args.spec {
        Some(path) => {
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
        None => {
            let source = args
                .source
                .as_ref()
                .expect("clap requires source unless --spec is set");
            import::discover(source, &args.objects, args.to.as_deref(), args.id.as_deref()).await?
        }
    };
    // A filter is written in one object's terms; broadcast to a glob it would
    // fail on the first table missing the column.
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

    // Dry runs render the `<column>` placeholder as a template to capture and
    // fill in; only a real import insists on a resolved id.
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

    // Credentials live on the command line, never in a spec: one source for the
    // run — the CLI uri, or path-free files, the only source a spec can carry.
    let source = match args.source.as_ref() {
        Some(uri) => import::connect(uri).await?,
        None => {
            // Fail before any prompt: every `from` must be a file locator.
            for target in spec.collections.values() {
                Duckdb::file(&target.from)?;
            }
            Source::Duck(Duckdb::Files)
        }
    };

    if args.dry_run {
        print!("{}", render(&spec, None));
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
    eprint!("{}", render(&spec, Some(&fresh)));
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

    let progress = output.spinner("importing");
    // Concurrent collections share the spinner; per-collection messages would
    // fight over it, so several report one total.
    let total_rows = std::sync::atomic::AtomicUsize::new(0);
    let count = spec.collections.len();
    let outcomes: Result<Vec<_>, Error> = futures::stream::iter(spec.collections.iter())
        .map(|(name, target)| {
            let schema = pending.remove(name.as_str());
            // The `move` callback takes references, not the spinner itself.
            let (progress, total_rows) = (&progress, &total_rows);
            import::load(&client, name, &source, target, schema, args.continue_on_error, move |n| {
                let total = total_rows.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                progress.set_message(match count {
                    1 => format!("{name}: {n} rows"),
                    _ => format!("{count} collections: {total} rows"),
                })
            })
            .map_ok(move |outcome| (name, outcome))
        })
        .buffer_unordered(CONCURRENCY)
        .try_collect()
        .await;
    progress.finish();
    let outcomes = outcomes.map_err(|e| match e.skippable() {
        // Documents already written stay written; a re-run skips them.
        true => Error::InvalidArgument(format!(
            "{e}\nnothing else was imported — fix the source and re-run \
             (documents already imported are skipped)"
        )),
        false => e,
    })?;

    if matches!(output.format, OutputFormat::Json) {
        let json: BTreeMap<_, _> = outcomes.iter().map(|(name, outcome)| (name, outcome)).collect();
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
    match outcome.failed {
        0 => format!("{} rows written", outcome.rows),
        failed => format!("{} rows written, {failed} failed", outcome.rows),
    }
}

/// The spec is the plan: what prints here is what `--spec` would re-run.
/// `fresh` is None before a cluster has been consulted, as in --dry-run.
fn render(spec: &Spec, fresh: Option<&[String]>) -> String {
    let mut out = String::new();
    let mut indexed: Vec<String> = Vec::new();
    for (name, target) in spec.collections.iter() {
        if let Some(fresh) = fresh {
            let state = match fresh.iter().any(|created| created == name) {
                true => "will create",
                false => "exists",
            };
            out.push_str(&format!("# {state}\n"));
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
            out.push_str(&format!("{} = {inline}{}\n", key(field), hint(spec)));
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
            "# no indexes declared — the data will import but will not be searchable;\n\
             # add an `index` to a field to index it\n",
        ),
        false => out.push_str(&format!("# indexed: {}\n", indexed.join(", "))),
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

fn hint(field: &import::Field) -> &'static str {
    if field.index.is_some() {
        return "";
    }
    match field.ty {
        import::Type::Text => r#"  # index = "keyword" | "exact" | "semantic""#,
        ty if ty.is_vector() || ty.is_sparse() => {
            r#"  # index = { vector = { metric = "cosine" } }"#
        }
        ty if ty.is_matrix() => "  # index = { multi_vector = {} }",
        import::Type::FloatList => {
            "  # not searchable as-is; for vector search use: \
             { type = \"f32_vector\", dim = <N>, index = { vector = { metric = \"cosine\" } } }"
        }
        _ => "",
    }
}

async fn preview(name: &str, source: &import::Source, target: &Target) -> Result<(), Error> {
    // Cap at the source rather than dropping the stream mid-read: reads only
    // the preview, and a dropped ES stream would leak its point-in-time.
    let cap = PREVIEW_ROWS as u64;
    let target = Target {
        limit: Some(target.limit.map_or(cap, |limit| limit.min(cap))),
        ..target.clone()
    };
    let mut rows = import::documents(source, &target).await?;
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
            .map(|(key, value)| Ok((key, serde_json::Value::try_from(value)?)))
            .collect::<Result<Vec<(String, serde_json::Value)>, topk_rs::Error>>()?;
        pairs.sort_by_key(|(k, _)| (k != ID, k.clone()));
        let doc = serde_json::Value::Object(pairs.into_iter().collect());
        // Docs go to stderr so `--dry-run > spec.toml` captures the spec alone.
        match std::io::stderr().is_terminal() {
            true => eprintln!("{}", elide(&doc)),
            false => eprintln!("{doc}"),
        }
    }
    if shown as u64 == cap {
        eprintln!("# … showing the first {PREVIEW_ROWS} rows");
    }
    Ok(())
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
