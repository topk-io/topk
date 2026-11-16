use futures::TryStreamExt;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::Args;

use crate::client::ManagementClient;
use crate::config::Config;
use crate::import::plan::{FieldSelection, Plan, Rename};
use crate::import::source::{Source, Uri};
use crate::import::state::{Run, RunId, State};
use crate::import::{Error, Import, WriteOptions};
use crate::output::Output;
use crate::scope::ScopeArgs;
use crate::util::Interrupted;

mod field;
mod output;

use output::ImportOutput;

#[derive(clap::Args, Debug)]
pub struct PlanArgs {
    #[arg(help = "Source objects to import; exact names, globs, or <object>=<collection>")]
    pub objects: Vec<String>,

    #[arg(
        short = 'f',
        long = "field",
        value_name = "FIELD",
        value_delimiter = ',',
        help = "Import only these fields; name, name=semantic|keyword|ngram|image:semantic, or name=vector:<dim>:<metric>. Repeatable"
    )]
    pub fields: Vec<FieldSelection>,
    #[arg(
        long = "rename",
        value_name = "SOURCE=TARGET",
        help = "Rename a source column before selecting fields. Repeatable"
    )]
    pub renames: Vec<Rename>,
    #[arg(
        long,
        value_name = "COLLECTION",
        help = "Name the target collection; required when it cannot be derived"
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
    #[arg(long, help = "Read at most this many rows per object")]
    pub limit: Option<u64>,
}

impl PlanArgs {
    pub async fn build(&self, source: &Source) -> Result<Plan, Error> {
        let catalog = source.catalog().await?;
        let mut plan = catalog
            .discover(&self.objects, self.to.as_deref(), self.id.as_deref())
            .map_err(|error| match error {
                Error::MissingId(name) => Error::InvalidArgument(format!(
                    "{} — pass --id <column>",
                    Error::MissingId(name)
                )),
                error => Error::InvalidArgument(format!("source {}: {error}", source.locator())),
            })?;
        if self.filter.is_some() && plan.collections.len() > 1 {
            return Err(Error::InvalidArgument(format!(
                "--filter applies to a single object, but {} objects matched — import them separately",
                plan.collections.len()
            )));
        }
        if (!self.fields.is_empty() || !self.renames.is_empty()) && plan.collections.len() > 1 {
            return Err(Error::InvalidArgument(
                "-f and --rename apply to one collection; import matched objects separately".into(),
            ));
        }
        for target in plan.collections.values_mut() {
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
        Ok(plan)
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
        short = 'b',
        long,
        value_name = "SIZE",
        default_value = "1MiB",
        help = "Target document bytes per partition batch"
    )]
    pub batch_bytes: bytesize::ByteSize,
}

#[derive(Args, Debug)]
#[command(
    override_usage = "topk import [OPTIONS] [SOURCE] [OBJECTS]...",
    after_help = "Field choices (-f name=<choice>):\n  types    text int float bool bytes image timestamp text_list int_list float_list\n  indexes  keyword semantic ngram, alone or after a type (text_list:keyword, image:semantic)\n  vectors  vector[:<dim>][:cosine|euclidean|dot_product|hamming], matrix[:<cols>][:maxsim], sparse[:dot_product]"
)]
pub struct ImportArgs {
    #[arg(
        required_unless_present = "resume",
        help = "Source URL, file path, or glob (e.g. topk://<collection> to copy a collection); with --resume, provide it only when credentials were in the source URL. \
                {name} in a glob makes that path segment a column ('data/{lang}/*.parquet'), \
                as do key=value folders ('data/lang=sw/part-0.parquet')"
    )]
    pub source: Option<Uri>,
    #[arg(
        long,
        value_name = "RUN",
        conflicts_with_all = ["objects", "to", "id", "filter", "limit", "partition", "fields", "renames", "dry_run"],
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
    tracing::info!(?args, "import");
    let mut ui = ImportOutput::new(output);
    let directory = match std::env::var_os("TOPK_IMPORT_STATE_DIR") {
        Some(path) => PathBuf::from(path),
        None => Config::dir()?.join("import"),
    };
    let import = if let Some(id) = args.resume {
        let run = Run::load(&directory, id).await?;
        if run.state().pending().next().is_none() {
            ui.message(format_args!(
                "run {}: all {} collection(s) already imported",
                run.id,
                run.state().plan.collections.len()
            ))?;
            if let Err(error) = run.remove() {
                tracing::warn!(%error, "cannot remove run state");
            }
            return ui.finish();
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
        let client = config.data_client(scope.project_id.clone(), &scope.region)?;
        let to_create = plan
            .resolve(&source, &client, &args.plan.fields, &args.plan.renames)
            .await?;
        if args.dry_run {
            ui.plan(&plan, &scope)?;
            for (name, target) in &plan.collections {
                ui.sample_header(name)?;
                let target = target.capped(5);
                let mut rows = source.documents(&target).await?;
                while let Some((partition, document)) = rows.try_next().await? {
                    ui.sample(partition.as_deref(), document)?;
                }
            }
            return Ok(ExitCode::SUCCESS);
        }
        let run = Run::start(
            &directory,
            State::new(source.locator().to_string(), scope, plan),
        )
        .await?;
        Import::start(source, client, run, to_create).await?
    };
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
