use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

use topk::client::{make_client, make_global_client};
use topk::commands::{ask, dataset, delete, list, login, logout, search, upload};
use topk::config;
use topk::datasets::{ensure_unique_region, get_region, make_cached_datasets_client};
use topk::output::{Output, OutputFormat};

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
    #[arg(long, env = "TOPK_HOST", global = true, hide = true)]
    host: Option<String>,

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

    /// Remove auth credentials
    Logout,

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

async fn run(cli: Cli, output: &Output) -> Result<()> {
    let mut cfg = config::load();
    let result = match cli.command {
        Some(Commands::Login) => run_login(cli, output, &mut cfg).await,
        Some(Commands::Logout) => run_logout(output, &mut cfg),
        command => {
            let api_key = cli.api_key.clone();
            let host = cli.host.clone().unwrap_or_else(|| "topk.io".to_string());
            let https = cli.https;
            run_command(command, api_key, &host, https, output, &cfg).await
        }
    };

    config::save(&cfg)?;
    result
}

async fn run_login(cli: Cli, output: &Output, cfg: &mut config::Config) -> Result<()> {
    let host = cli.host.unwrap_or_else(|| "topk.io".to_string());
    let https = cli.https;
    output.print(&login::run(cli.api_key, cfg, &host, https, output)?)?;
    Ok(())
}

fn run_logout(output: &Output, cfg: &mut config::Config) -> Result<()> {
    output.print(&logout::run(cfg))?;
    Ok(())
}

fn require_api_key(api_key: Option<String>, cfg: &config::Config) -> Result<String> {
    login::resolve(api_key, cfg)?.ok_or_else(|| {
        anyhow::anyhow!(
            "API key not set. Set TOPK_API_KEY environment variable or run: `topk login`"
        )
    })
}

async fn run_command(
    command: Option<Commands>,
    api_key: Option<String>,
    host: &str,
    https: bool,
    output: &Output,
    cfg: &config::Config,
) -> Result<()> {
    match command {
        Some(Commands::Completions { shell }) => {
            generate(shell, &mut Cli::command(), "topk", &mut std::io::stdout());
            Ok(())
        }

        Some(Commands::Dataset { action }) => {
            let api_key = require_api_key(api_key, cfg)?;
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
            let api_key = require_api_key(api_key, cfg)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&upload::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Delete(args)) => {
            let api_key = require_api_key(api_key, cfg)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&delete::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::List(args)) => {
            let api_key = require_api_key(api_key, cfg)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            // JSON mode streams NDJSON progressively inside run(); only render the table in non-json mode.
            let result = list::run(&client, &args, output).await?;
            if !output.is_json() {
                output.print_text(&result)?;
            }
            Ok(())
        }

        Some(Commands::Ask(args)) => {
            let api_key = require_api_key(api_key, cfg)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&ask::run(&client, &args, output).await?)?;
            Ok(())
        }

        Some(Commands::Search(args)) => {
            let api_key = require_api_key(api_key, cfg)?;
            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&search::run(&client, &args, output).await?)?;
            Ok(())
        }

        None => {
            Cli::command().print_help()?;
            Ok(())
        }

        Some(Commands::Login | Commands::Logout) => unreachable!(),
    }
}
