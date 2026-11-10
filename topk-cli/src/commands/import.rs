use std::process::ExitCode;

use clap::Args;
use indicatif::{MultiProgress, ProgressDrawTarget};

use crate::config::Config;
use crate::endpoint::DataEndpoint;
use crate::import::{self, Destination, Error, Import, PlanOptions, Run, RunId, Uri, WriteOptions};
use crate::output::Output;
use crate::ProjectId;

mod output;

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
        help = "Continue the saved plan from its last checkpoint, by the id in its header"
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
    pub plan: PlanOptions,
    #[command(flatten)]
    pub write: WriteOptions,

    /// Project to import into; defaults to the currently selected project
    #[arg(long, env = "TOPK_PROJECT_ID", help_heading = "Connection options")]
    pub project_id: Option<ProjectId>,

    #[command(flatten)]
    pub data: DataEndpoint,
}

pub async fn run(config: Config, args: &ImportArgs, output: Output) -> anyhow::Result<ExitCode> {
    tracing::info!(?args, "import");
    let config = config.with_project(args.project_id.clone());
    let source_uri = args.source.clone();
    let import = if let Some(id) = args.resume {
        let run = Run::load(id).await?;
        if run.pending().next().is_none() {
            let code = output::completed(&run, output)?;
            run.remove();
            return Ok(code);
        }
        Import::resume(&config, run, source_uri.as_ref()).await?
    } else {
        let id_hint = if args.plan.spec.is_some() {
            "set `id` in the spec"
        } else {
            "pass --id <column> (or set `id` in a spec)"
        };
        // Credentials never enter a spec: the CLI uri is the source, or every `from`
        // is its own file locator.
        let source = source_uri
            .clone()
            .unwrap_or_default()
            .build(&config)
            .await?;
        let spec = args.plan.build(&source).await?;
        if args.dry_run {
            output::preview(&source, &spec, id_hint).await?;
            return Ok(ExitCode::SUCCESS);
        }
        let destination = Destination::build(&config, &args.data)?;
        Import::new(&config, source, spec, destination)
            .await
            .map_err(|e| match e {
                Error::MissingId(_) => anyhow::anyhow!(
                    "{e} — {} (it becomes each document's `{}`)",
                    id_hint,
                    import::ID
                ),
                e => e.into(),
            })?
    };
    if !output::confirm(&import.run, args.yes || args.resume.is_some())? {
        return Ok(ExitCode::SUCCESS);
    }

    let progress = match output {
        Output::Text => MultiProgress::new(),
        Output::Json => MultiProgress::with_draw_target(ProgressDrawTarget::hidden()),
    };
    if output == Output::Text {
        import::set_progress(progress.clone());
    }
    let resume_hint = format!(
        "Continue with: topk import {}--resume {}",
        if args.source.as_ref().and_then(Uri::password).is_some() {
            "SOURCE "
        } else {
            ""
        },
        import.run.id,
    );
    let outcomes = tokio::select! {
        outcomes = import.execute(&progress, &args.write) => outcomes,
        _ = tokio::signal::ctrl_c() => {
            let _ = progress.clear();
            eprintln!("stopped; rows written so far stay. {resume_hint}");
            return Ok(ExitCode::from(130));
        }
    };
    let outcomes = outcomes.inspect_err(|e| {
        if matches!(e, Error::Topk(topk_rs::Error::DocumentValidationError(_))) {
            eprintln!("Use --continue-on-error to skip rejected documents.");
        }
        eprintln!("{resume_hint}");
    });
    let outcomes = outcomes?;
    output::report(&outcomes, output)
}
