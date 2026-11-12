use futures::TryStreamExt;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Args;

use crate::client::{DataClient, ManagementClient};
use crate::config::Config;
use crate::import::source::{Source, Uri};
use crate::import::spec::Spec;
use crate::import::state::{Run, RunId, State};
use crate::import::{Error, Import, WriteOptions};
use crate::management::proto::GetCollectionRequest;
use crate::output::Output;
use crate::scope::{Scope, ScopeArgs};
use crate::util::Interrupted;

mod output;

use output::ImportOutput;

#[derive(clap::Args, Debug)]
pub struct PlanArgs {
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
    #[arg(
        long,
        value_name = "COLUMN",
        help = "Column whose value is the partition; each row goes to its own"
    )]
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
}

impl PlanArgs {
    pub async fn build(&self, source: &Source) -> Result<Spec, Error> {
        let mut spec = match &self.spec {
            Some(path) => {
                toml::from_str::<Spec>(&std::fs::read_to_string(path).map_err(|error| {
                    Error::Setup(
                        anyhow::Error::new(error)
                            .context(format!("cannot read spec {}", path.display())),
                    )
                })?)?
            }
            None => {
                let catalog = source.catalog().await?;
                catalog
                    .discover(&self.objects, self.to.as_deref(), self.id.as_deref())
                    .map_err(|error| match error {
                        Error::MissingId(name) => Error::InvalidArgument(format!(
                            "{} — pass --id <column> (or set `id` in a spec)",
                            Error::MissingId(name)
                        )),
                        error => {
                            Error::InvalidArgument(format!("source {}: {error}", source.locator()))
                        }
                    })?
            }
        };
        if self.filter.is_some() && spec.collections.len() > 1 {
            return Err(Error::InvalidArgument(format!(
                "--filter applies to a single object, but {} objects matched — set `filter` per collection in a spec",
                spec.collections.len()
            )));
        }
        for target in spec.collections.values_mut() {
            if let Some(filter) = &self.filter {
                target.filter = Some(filter.clone());
            }
            if let Some(limit) = self.limit {
                target.limit = Some(limit);
            }
            if let Some(partition) = &self.partition {
                target.partition = Some(partition.clone());
            }
        }
        Ok(spec)
    }
}

#[derive(clap::Args, Debug)]
pub struct WriteArgs {
    #[arg(
        long,
        help = "Skip row decoding, conversion, and document-validation failures; operational failures still stop the run. Exits nonzero if any rows were skipped"
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
        help = "Target document bytes per partition batch"
    )]
    pub batch_bytes: bytesize::ByteSize,
}

#[derive(Args, Debug)]
// Clap's generated usage renders `<SOURCE>` as required; it is not, with --spec.
#[command(override_usage = "topk import [OPTIONS] [SOURCE] [OBJECTS]...")]
pub struct ImportArgs {
    #[arg(
        required_unless_present_any = ["spec", "resume"],
        help = "Source URL, file path, or glob (e.g. topk://<collection> to copy a collection); required with --spec except for files, or with --resume when credentials were in the source URL. \
                {name} in a glob makes that path segment a column ('data/{lang}/*.parquet'), \
                as do key=value folders ('data/lang=sw/part-0.parquet')"
    )]
    pub source: Option<Uri>,
    #[arg(
        long,
        value_name = "RUN",
        conflicts_with_all = ["objects", "to", "id", "filter", "limit", "partition", "spec", "dry_run"],
        help = "Continue the saved plan from its last checkpoint, using the printed resume command"
    )]
    pub resume: Option<RunId>,
    #[arg(short = 'y', long, help = "Skip confirmation")]
    pub yes: bool,
    #[arg(
        long,
        conflicts_with = "resume",
        help = "Print the spec and a sample of documents, without importing"
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
    tracing::info!(?args, "import");
    let mut ui = ImportOutput::new(output);
    let directory = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
        Some(path) => PathBuf::from(path),
        None => Config::dir()?.join("import"),
    };
    let (source, run) = if let Some(id) = args.resume {
        let run = Run::load(&directory, id).await?;
        if run.state().pending().next().is_none() {
            ui.message(format_args!(
                "run {}: all {} collection(s) already imported",
                run.id,
                run.state().spec.collections.len()
            ))?;
            if let Err(error) = run.remove() {
                tracing::warn!(%error, "cannot remove run state");
            }
            return ui.finish();
        }
        let uri = run.state().source(args.source.as_ref())?;
        (args.source(uri, &config).await?, run)
    } else {
        let source = args
            .source(args.source.clone().unwrap_or(Uri::Files(None)), &config)
            .await?;
        let spec = args.plan.build(&source).await?;
        if args.dry_run {
            ui.spec(&spec)?;
            for (name, target) in &spec.collections {
                ui.sample_header(name)?;
                let target = target.capped(5);
                let mut rows = source.documents(&target).await?;
                while let Some((partition, document)) = rows.try_next().await? {
                    ui.sample(partition.as_deref(), document)?;
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        let destination = args.scope.resolve(&config).await?;
        let run = Run::start(
            &directory,
            State::new(source.locator().to_string(), destination, spec),
        )
        .await?;
        (source, run)
    };
    let client = DataClient::new(&config, run.state().destination.clone())?;
    let import = Import::prepare(source, (*client).clone(), run).await?;
    if !ui.confirm(import.run(), args.yes || args.resume.is_some())? {
        return Ok(ExitCode::SUCCESS);
    }
    let resume_hint = format!(
        "Continue with: topk import {}--resume {}",
        if args.source.as_ref().and_then(Uri::password).is_some() {
            "SOURCE "
        } else {
            ""
        },
        import.run().id
    );
    let options = WriteOptions {
        continue_on_error: args.write.continue_on_error,
        concurrency: args.write.concurrency,
        batch_bytes: args.write.batch_bytes,
    };
    let execution = import.execute(options);
    futures::pin_mut!(execution);
    loop {
        let event = tokio::select! {
            event = execution.try_next() => event,
            _ = tokio::signal::ctrl_c() => {
                ui.message(format_args!("stopped; rows written so far stay. {resume_hint}"))?;
                return Err(Interrupted.into());
            }
        };
        match event {
            Ok(Some(event)) => ui.event(event)?,
            Ok(None) => break,
            Err(error) => {
                if matches!(error, Error::RejectedDocuments(_)) {
                    let _ = ui.message("Use --continue-on-error to skip rejected documents.");
                }
                let _ = ui.message(&resume_hint);
                return Err(error.into());
            }
        }
    }
    ui.finish()
}

impl ImportArgs {
    async fn source(&self, mut uri: Uri, config: &Config) -> anyhow::Result<Source> {
        if let Uri::Topk {
            project,
            collection: name,
        } = &mut uri
        {
            let resolved = match project.as_ref() {
                Some(project) => project.clone(),
                None => self.scope.project_id(config).await?,
            };
            *project = Some(resolved.clone());
            let name = name.clone();
            let collection = ManagementClient::new(config.clone())?
                .collections
                .get_collection(GetCollectionRequest {
                    project_id: resolved.to_string(),
                    name: name.clone(),
                })
                .await?
                .into_inner()
                .collection
                .ok_or_else(|| anyhow::anyhow!("management returned no source collection"))?;
            anyhow::ensure!(
                !collection.region.is_empty(),
                "source collection has no region"
            );
            let client = DataClient::new(
                config,
                Scope {
                    project_id: resolved,
                    region: collection.region,
                },
            )?;
            return Ok(Source::topk(
                (*client).clone(),
                uri.to_string(),
                name,
                collection.schema,
            ));
        }
        Ok(uri.connect().await?)
    }
}
