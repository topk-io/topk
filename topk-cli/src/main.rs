use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

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
    let Cli {
        command,
        api_key,
        host,
        https,
        output: _,
    } = cli;

    let mut cfg = config::load();

    match command {
        Some(Commands::Login) => {
            let result = login::run(&host, https)?;
            if let Some(api_key) = &result.api_key {
                cfg.api_key = Some(api_key.clone());
                config::save(&cfg)?;
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
                cfg.api_key = None;
                config::save(&cfg)?;
            }
            output.print(&result)?;
            Ok(())
        }

        Some(Commands::Dataset { action }) => {
            let api_key = require_api_key(api_key)?;
            let client = make_cached_datasets_client(make_global_client(&api_key, &host, https));

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
            let api_key = require_api_key(api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&upload::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Delete(args)) => {
            let api_key = require_api_key(api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&delete::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::List(args)) => {
            let api_key = require_api_key(api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            // JSON mode streams NDJSON progressively inside run(); only render the table in non-json mode.
            let result = list::run(&client, &args, output).await?;
            if !output.is_json() {
                output.print(&result)?;
            }
            Ok(())
        }

        Some(Commands::Ask(args)) => {
            let api_key = require_api_key(api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&ask::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Search(args)) => {
            let api_key = require_api_key(api_key)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &host, https);

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
