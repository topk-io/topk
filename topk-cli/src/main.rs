use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use futures::TryStreamExt;
use tokio_stream::StreamExt;

use topk::client::{make_client, make_global_client};
use topk::commands::{ask, dataset, delete, list, login, logout, search, upload};
use topk::config;
use topk::datasets::{ensure_unique_region, get_region, make_cached_datasets_client};
use topk::output::{Output, OutputFormat};
use topk_rs::Error;

#[derive(Parser)]
#[command(name = "topk", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// TopK API key (overrides TOPK_API_KEY environment variable)
    #[arg(
        long,
        env = "TOPK_API_KEY",
        global = true,
        hide_env_values = true,
        hide = true
    )]
    api_key: Option<String>,

    /// Host (overrides TOPK_HOST environment variable, default: topk.io)
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
        global = true,
        hide = true
    )]
    host: String,

    #[arg(
        long,
        env = "TOPK_HTTPS",
        default_value = "true",
        global = true,
        hide = true
    )]
    https: bool,

    /// Output format
    #[arg(short = 'o', long, default_value = "text", global = true)]
    output: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in by entering your API key
    Login,

    /// Get a grounded answer from documents with source citations for a query
    Ask(ask::AskArgs),

    /// Find relevant passages in documents for a query
    Search(search::SearchArgs),

    /// Upload files
    Upload(upload::UploadArgs),

    /// Delete a document
    Delete(delete::DeleteArgs),

    /// List documents in a dataset
    List(list::ListArgs),

    /// Manage datasets (create, list, delete)
    Dataset {
        #[command(subcommand)]
        action: dataset::DatasetAction,
    },

    /// Remove auth credentials
    Logout,

    /// Generate shell completion script
    #[command(hide = true)]
    Completions { shell: Shell },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let output = Output::new(cli.output);

    match run(cli, &output).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            output.error(&e);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli, output: &Output) -> Result<(), Error> {
    let cfg = config::load();

    match cli.command {
        Some(Commands::Login) => {
            let result = login::run(&cli.host, cli.https)?;

            if let Some(api_key) = &result.api_key {
                config::set_api_key(api_key.clone())?;
                if output.is_json() {
                    output.print(&result)?;
                } else {
                    output.success("API key saved.");
                }
            } else {
                output.print(&result)?;
            }

            Ok(())
        }

        Some(Commands::Logout) => {
            let result = logout::run(&cfg);
            if result.cleared {
                config::clear()?;
            }
            output.print(&result)?;
            Ok(())
        }

        Some(Commands::Dataset { action }) => {
            let api_key = require_api_key(cli.api_key)?;
            let client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            match action {
                dataset::DatasetAction::List => {
                    output.print(&dataset::list(client).await?)?;
                }
                dataset::DatasetAction::Get { dataset: name } => {
                    output.print(&dataset::get(client, &name).await?)?;
                }
                dataset::DatasetAction::Create(args) => {
                    output.print(&dataset::create(client, &args).await?)?;
                }
                dataset::DatasetAction::Delete(args) => {
                    output.print(&dataset::delete(client, &args, output).await?)?;
                }
            }

            Ok(())
        }

        Some(Commands::Upload(args)) => {
            let api_key = require_api_key(cli.api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            output.print(&upload::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Delete(args)) => {
            let api_key = require_api_key(cli.api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            output.print(&delete::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::List(args)) => {
            let api_key = require_api_key(cli.api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            let stream = list::run(&client, &args).await?;

            if output.is_json() {
                tokio::pin!(stream);
                while let Some(entry) = stream.next().await {
                    output
                        .print_json_line(&list::ListEntryRow::from(entry?))
                        .map_err(|e| Error::Internal(e.to_string()))?;
                }
            } else {
                let entries = stream
                    .map(|entry| entry.map(list::ListEntryRow::from))
                    .try_collect()
                    .await?;
                output.print(&list::ListResult { entries })?;
            }
            Ok(())
        }

        Some(Commands::Ask(args)) => {
            let api_key = require_api_key(cli.api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));
            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            output.print(&ask::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Search(args)) => {
            let api_key = require_api_key(cli.api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));
            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            output.print(&search::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Completions { shell }) => {
            generate(shell, &mut Cli::command(), "topk", &mut std::io::stdout());
            Ok(())
        }

        None => {
            Cli::command().print_help()?;
            Ok(())
        }
    }
}

fn require_api_key(api_key: Option<String>) -> Result<String, Error> {
    let cfg = config::load();
    login::resolve(api_key, &cfg)?.ok_or_else(|| {
        Error::Input(anyhow::anyhow!(
            "API key not set. Set TOPK_API_KEY environment variable or run: `topk login`"
        ))
    })
}
