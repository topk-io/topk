use futures::TryStreamExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use topk_rs::Error as SdkError;

use anyhow::Context;
use clap::builder::ArgPredicate;
use clap::Args;
use tracing::{info, warn};

use crate::client::ManagementClient;
use crate::config::Config;
use crate::import::plan::{collection_name, FieldSelection, Plan, Rename, Target};
use crate::import::source::{Source, Uri};
use crate::import::state::{Run, RunId, State};
use crate::import::{Aimd, Error, Import, WriteOptions};
use crate::output::Output;
use crate::scope::ScopeArgs;
use crate::util::Interrupted;

mod field;
mod output;

use output::ImportOutput;

#[derive(clap::Args, Debug)]
pub struct PlanArgs {
    #[arg(
        short = 'f',
        long = "field",
        value_name = "FIELD",
        value_delimiter = ',',
        help = "Set a field's type or index: name=semantic|keyword|ngram|image:semantic, name=vector:<dim>:<metric>, or a type (choices below). Every other field imports as discovered. Repeatable"
    )]
    pub fields: Vec<FieldSelection>,
    #[arg(
        long,
        value_name = "FIELD",
        value_delimiter = ',',
        help = "Leave these fields out. Repeatable"
    )]
    pub exclude: Vec<String>,
    #[arg(
        long = "rename",
        value_name = "SOURCE=TARGET",
        help = "Rename a source column before --exclude and -f apply. Repeatable"
    )]
    pub renames: Vec<Rename>,
    #[arg(
        long,
        value_name = "COLLECTION",
        required_unless_present = "resume",
        value_parser = collection_name,
        help = "Collection to import into; created when it does not exist"
    )]
    pub to: Option<String>,
    #[arg(
        long,
        value_name = "COLUMN",
        help = "Column to use as the document id (_id); use when it can't be auto-detected"
    )]
    pub id: Option<String>,
    #[arg(
        long,
        value_name = "COLUMN",
        help = "Column whose value is the partition; each row goes to its own"
    )]
    pub partition: Option<String>,
    #[arg(
        long,
        help = "Only read rows matching a filter, in the source's language: \
                SQL WHERE for files and databases, a JSON query for mongodb/elasticsearch"
    )]
    pub filter: Option<String>,
    #[arg(long, help = "Read at most this many rows")]
    pub limit: Option<u64>,
}

impl PlanArgs {
    pub async fn build(&self, source: &Source) -> Result<Plan, Error> {
        let collection = self
            .to
            .clone()
            .ok_or_else(|| Error::InvalidArgument("--to <collection> is required".into()))?;
        let table = source.discover().await.map_err(|error| {
            match error.to_string().contains(source.locator()) {
                true => error,
                false => anyhow::Error::new(error)
                    .context(format!("source {}", source.locator()))
                    .into(),
            }
        })?;
        let mut target = Target::new(table, self.id.as_deref())?;
        target.filter = self.filter.clone();
        target.limit = self.limit;
        target.partition = self.partition.clone();
        Ok(Plan { collection, target })
    }
}

#[derive(clap::Args, Debug)]
pub struct WriteArgs {
    #[arg(
        long,
        help = "Skip row decoding, conversion, and document-validation failures, listing each in a file next to the run state; operational failures still stop the run"
    )]
    pub continue_on_error: bool,
    #[arg(
        short = 'c',
        long,
        value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..=1024),
        default_value = "64",
        default_value_if("resume", ArgPredicate::IsPresent, None::<&str>),
        help = "Most upserts in flight across the whole run; the window starts at 1 and adapts up to this"
    )]
    pub concurrency: Option<usize>,
    #[arg(
        short = 'b',
        long,
        value_name = "SIZE",
        default_value = "1MiB",
        default_value_if("resume", ArgPredicate::IsPresent, None::<&str>),
        value_parser = batch_bytes,
        help = "Target document bytes per partition batch"
    )]
    pub batch_bytes: Option<bytesize::ByteSize>,
    #[arg(long, hide = true, default_value = "8", value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..=1024))]
    pub window_start: usize,
    #[arg(long, hide = true, default_value = "0.9", value_parser = window_cut)]
    pub window_cut: f64,
    #[arg(long, hide = true, default_value = "0.1", value_parser = window_grow)]
    pub window_grow: f64,
    #[arg(long, hide = true, default_value = "0.05", value_parser = window_grow)]
    pub window_gain: f64,
    #[arg(long, hide = true, default_value = "3", value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..=64))]
    pub window_rounds: usize,
}

impl WriteArgs {
    /// With `--resume`, a flag left out keeps the saved run's value.
    fn options(&self, saved: Option<&WriteOptions>) -> anyhow::Result<WriteOptions> {
        Ok(WriteOptions {
            continue_on_error: self.continue_on_error
                || saved.is_some_and(|saved| saved.continue_on_error),
            concurrency: self
                .concurrency
                .or(saved.map(|saved| saved.concurrency))
                .context("--concurrency is required")?,
            batch_bytes: self
                .batch_bytes
                .map(|size| size.as_u64())
                .or(saved.map(|saved| saved.batch_bytes))
                .context("--batch-bytes is required")?,
        })
    }
}

#[derive(Args, Debug)]
#[command(
    override_usage = "topk import [OPTIONS] --to <COLLECTION> <SOURCE>\n       topk import --resume <RUN>",
    after_help = "Field choices (-f name=<choice>):\n  types    text int float bool bytes image timestamp text_list int_list float_list\n  indexes  keyword semantic ngram, alone or after a type (text_list:keyword, image:semantic)\n  vectors  vector[:<dim>][:cosine|euclidean|dot_product|hamming], matrix[:<cols>][:maxsim], sparse[:dot_product]\n  A list of floats imports as float_list; choose vector to store and index an embedding\n\nField names cannot start with _: a source field _lang imports as lang, or as the name --rename gives it\n\nGlobs: * matches files in one folder, ** also matches subfolders ('./data/**/*.csv')\n\nExamples:\n  topk import ./books.parquet --to books --dry-run\n  topk import './data/*.csv' --to books --id isbn -y\n  topk import books.jsonl --to books -f title=keyword,body=semantic\n  topk import docs.parquet --to docs -f embedding=vector:cosine\n  topk import postgres://user:pw@host/db#orders --to orders\n  topk import mongodb://host/shop#products --to products\n  topk import sqlite:./shop.db#orders --to orders\n  topk import es://localhost:9200/articles --to articles\n  topk import topk://books --to books_v2"
)]
pub struct ImportArgs {
    #[arg(
        required_unless_present = "resume",
        help = "Source URL, file path, or glob (e.g. topk://<collection> to copy a collection); with --resume, provide it only when credentials were in the source URL. \
                {name} in a glob makes that path segment a column ('data/{lang}/*.parquet'), \
                as do key=value folders ('data/lang=sw/part-0.parquet')"
    )]
    pub source: Option<Uri>,
    #[arg(hide = true, value_parser = |object: &str| Err::<String, _>(format!(
        "one import reads one source; put the table, collection or index in its URI, e.g. postgres://host/db#{object} or es://host/{object}"
    )))]
    pub object: Option<String>,
    #[arg(
        long,
        value_name = "RUN",
        conflicts_with_all = ["to", "id", "filter", "limit", "partition", "fields", "exclude", "renames", "dry_run"],
        help = "Continue the saved plan from its last checkpoint, using the printed resume command"
    )]
    pub resume: Option<RunId>,
    #[arg(short = 'y', long, help = "Skip confirmation")]
    pub yes: bool,
    #[arg(
        long,
        conflicts_with = "resume",
        help = "Preview the import plan and a sample of documents, without importing"
    )]
    pub dry_run: bool,

    #[command(flatten)]
    pub plan: PlanArgs,
    #[command(flatten)]
    pub write: WriteArgs,

    #[command(flatten)]
    pub scope: ScopeArgs,
}

pub async fn run(config: Config, args: &ImportArgs, output: Output) -> anyhow::Result<ExitCode> {
    info!(?args, "import");
    let mut ui = ImportOutput::new(output);
    let directory = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
        Some(path) => PathBuf::from(path),
        None => Config::dir()?.join("import"),
    };
    let import = if let Some(id) = args.resume {
        let mut run = Run::load(&directory, id).await?;
        run.set_write(args.write.options(Some(&run.state().write))?);
        if run.state().done() {
            ui.message(format_args!(
                "run {}: {} already imported",
                run.id,
                run.state().plan.collection
            ))?;
            if let Err(error) = run.remove() {
                warn!(%error, "cannot remove run state");
            }
            ui.finish()?;
            return Ok(ExitCode::SUCCESS);
        }
        let uri = run.state().source(args.source.as_ref())?;
        let source = args.source(uri, &config).await?;
        let client = config.data_client(
            run.state().destination.project_id.clone(),
            &run.state().destination.region,
        )?;
        Import::resume(source, client, run).await?
    } else {
        let mut uri = args.source.clone().context("SOURCE is required")?;
        // A topk:// source without a project reads from the destination's project.
        let scope = match &mut uri {
            Uri::Topk { project, .. } if project.is_none() => {
                let scope = args.scope.resolve(&config).await?;
                *project = Some(scope.project_id.clone());
                Some(scope)
            }
            _ => None,
        };
        let source = args.source(uri, &config).await?;
        let mut plan = args.plan.build(&source).await?;
        let scope = match scope {
            Some(scope) => scope,
            None => args.scope.resolve(&config).await?,
        };
        if let Some(Uri::Topk {
            project,
            collection,
        }) = &args.source
        {
            anyhow::ensure!(
                *collection != plan.collection
                    || project.as_ref().is_some_and(|p| *p != scope.project_id),
                "topk://{collection} is the destination; import into another collection"
            );
        }
        let client = config.data_client(scope.project_id.clone(), &scope.region)?;
        let create = plan
            .resolve(
                &source,
                &client,
                &args.plan.fields,
                &args.plan.renames,
                &args.plan.exclude,
            )
            .await?;
        if args.dry_run {
            ui.plan(&plan, &scope)?;
            ui.sample_header(&plan.collection)?;
            let mut rows = source.documents(&plan.target.capped(5)).await?;
            while let Some((partition, document)) = rows.try_next().await? {
                ui.sample(partition.as_deref(), document)?;
            }
            return Ok(ExitCode::SUCCESS);
        }
        let run = Run::start(
            &directory,
            State::new(
                source.locator().to_string(),
                scope,
                plan,
                args.write.options(None)?,
            ),
        )
        .await?;
        Import::start(source, client, run, create).await?
    };
    if !ui.confirm(import.run(), args.yes || args.resume.is_some())? {
        return Ok(ExitCode::SUCCESS);
    }
    import.run().save()?;
    let resume = format!(
        "topk import {}--resume {}",
        if args.source.as_ref().and_then(Uri::password).is_some() {
            "SOURCE "
        } else {
            ""
        },
        import.run().id
    );
    ui.start(directory.join(format!("{}.rejected.jsonl", import.run().id)))?;
    let mut refresh = tokio::time::interval(Duration::from_millis(250));
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let window = import.run().state().write.concurrency;
    let execution = import.execute(Aimd {
        start: args.write.window_start,
        max: window,
        cut: args.write.window_cut,
        grow: args.write.window_grow,
        gain: args.write.window_gain,
        rounds: args.write.window_rounds,
    });
    let interrupt = tokio::signal::ctrl_c();
    futures::pin_mut!(execution, interrupt);
    loop {
        let event = tokio::select! {
            event = execution.try_next() => event,
            _ = refresh.tick() => {
                ui.tick();
                continue;
            }
            _ = &mut interrupt => {
                ui.message(format_args!("stopped; rows written so far stay. Continue with: {resume}"))?;
                return Err(Interrupted.into());
            }
        };
        match event {
            Ok(Some(event)) => ui.event(event)?,
            Ok(None) => break,
            Err(error) => {
                let _ = match error {
                    Error::Doc { .. }
                    | Error::Oversized { .. }
                    | Error::Topk(SdkError::DocumentValidationError(_)) => {
                        ui.message(format_args!(
                            "Skip rows that fail and continue with: {resume} --continue-on-error"
                        ))
                    }
                    _ => ui.message(format_args!("Continue with: {resume}")),
                };
                return Err(error.into());
            }
        }
    }
    ui.finish()?;
    Ok(ExitCode::SUCCESS)
}

impl ImportArgs {
    async fn source(&self, uri: Uri, config: &Config) -> anyhow::Result<Source> {
        if let Uri::Topk {
            project,
            collection: name,
        } = &uri
        {
            let project_id = project.clone().context("topk:// source has no project")?;
            let name = name.clone();
            let collection = ManagementClient::new(config.clone())?
                .collections()
                .get(&project_id, &name)
                .await?;
            anyhow::ensure!(
                !collection.region.is_empty(),
                "source collection has no region"
            );
            let client = config.data_client(project_id, &collection.region)?;
            return Ok(Source::topk(
                client,
                uri.to_string(),
                name,
                collection.schema,
            ));
        }
        Ok(uri.connect().await?)
    }
}

fn batch_bytes(raw: &str) -> Result<bytesize::ByteSize, String> {
    match raw.parse::<bytesize::ByteSize>()? {
        size if size.as_u64() == 0 => Err("must be at least 1B".into()),
        size => Ok(size),
    }
}

fn window_cut(raw: &str) -> Result<f64, String> {
    match raw.parse::<f64>().map_err(|e| e.to_string())? {
        cut if cut > 0.0 && cut < 1.0 => Ok(cut),
        _ => Err("must be between 0 and 1, exclusive".into()),
    }
}

fn window_grow(raw: &str) -> Result<f64, String> {
    match raw.parse::<f64>().map_err(|e| e.to_string())? {
        grow if grow.is_finite() && grow > 0.0 => Ok(grow),
        _ => Err("must be positive".into()),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use rstest::rstest;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        import: ImportArgs,
    }

    #[rstest]
    #[case::defaults(&["x.parquet", "--to", "c"], None, (false, 64, 1_048_576))]
    #[case::flags(&["x.parquet", "--to", "c", "-c", "2", "-b", "32"], None, (false, 2, 32))]
    #[case::resume_keeps_saved(&["--resume", "1234abcd"], Some((true, 2, 32)), (true, 2, 32))]
    #[case::resume_flag_replaces(&["--resume", "1234abcd", "-c", "4"], Some((true, 2, 32)), (true, 4, 32))]
    fn write_options(
        #[case] args: &[&str],
        #[case] saved: Option<(bool, usize, u64)>,
        #[case] expected: (bool, usize, u64),
    ) {
        let saved = saved.map(
            |(continue_on_error, concurrency, batch_bytes)| WriteOptions {
                continue_on_error,
                concurrency,
                batch_bytes,
            },
        );
        let cli = Cli::try_parse_from(["topk"].iter().chain(args).copied()).unwrap();
        let got = cli.import.write.options(saved.as_ref()).unwrap();
        assert_eq!(
            (got.continue_on_error, got.concurrency, got.batch_bytes),
            expected
        );
    }

    #[rstest]
    #[case::cut_one(&["--window-cut", "1"])]
    #[case::cut_nan(&["--window-cut", "NaN"])]
    #[case::grow_zero(&["--window-grow", "0"])]
    #[case::grow_infinite(&["--window-grow", "inf"])]
    fn window_flags_rejected(#[case] flags: &[&str]) {
        let args = ["topk", "x.parquet", "--to", "c"]
            .iter()
            .chain(flags)
            .copied();
        assert!(Cli::try_parse_from(args).is_err());
    }
}
