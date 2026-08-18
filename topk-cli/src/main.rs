use std::collections::HashMap;
use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use futures::TryStreamExt;
use tokio_stream::StreamExt;

use topk::client::{make_client, make_global_client};
use topk::commands::{ask, dataset, delete, list, login, search, upload};
use topk::config;
use topk::dataset_region_cache;
use topk::datasets::{ensure_unique_region, get_region, make_cached_datasets_client};
use topk::output::{is_broken_pipe, Output, OutputFormat};
use topk_rs::Error;

#[derive(Parser)]
#[command(name = "topk", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// TopK API key (or run `topk login`)
    #[arg(
        long,
        env = "TOPK_API_KEY",
        global = true,
        hide_env_values = true,
        help_heading = "Global options"
    )]
    api_key: Option<String>,

    /// API domain; the endpoint is <REGION>.api.<HOST>
    #[arg(
        long,
        env = "TOPK_HOST",
        default_value = "topk.io",
        global = true,
        help_heading = "Global options"
    )]
    host: String,

    /// Connect over HTTPS (default: true; TOPK_HTTPS=false to disable)
    #[arg(
        long,
        env = "TOPK_HTTPS",
        default_value = "true",
        global = true,
        help_heading = "Global options"
    )]
    https: bool,

    /// Region to write to; list available regions at https://docs.topk.io/regions
    #[arg(long, env = "TOPK_REGION", global = true, help_heading = "Global options")]
    region: Option<String>,

    /// Output format
    #[arg(
        short = 'o',
        long,
        default_value = "text",
        global = true,
        help_heading = "Global options"
    )]
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

    /// Bulk import from a database, file or object store
    Import(topk::commands::import::ImportArgs),

    /// Manage datasets (create, list, update, delete)
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
    let mut cli = Cli::parse();
    // Set-but-empty env vars (`TOPK_REGION=`) read as unset.
    cli.api_key = cli.api_key.filter(|v| !v.is_empty());
    cli.region = cli.region.filter(|v| !v.is_empty());

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
    let config = config::load();

    match cli.command {
        Some(Commands::Login) => {
            let api_key = match cli.api_key {
                Some(key) => Some(key),
                None => login::run(&cli.host, cli.https)?,
            };

            match api_key {
                Some(api_key) => {
                    config::set_api_key(api_key)?;
                    output.success("API key saved.");
                }
                None => {
                    output.print(&"Skipping authentication.")?;
                }
            }

            Ok(())
        }

        Some(Commands::Dataset { action }) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            match action {
                dataset::DatasetAction::List => {
                    let result = dataset::list(client).await?;
                    match output.format {
                        OutputFormat::Json => {
                            for dataset in &result.datasets {
                                if let Err(err) = output.print_json_line(dataset) {
                                    if is_broken_pipe(&err) {
                                        break;
                                    }
                                    return Err(err);
                                }
                            }
                        }
                        OutputFormat::Text => {
                            output.print(&result)?;
                        }
                    }
                }
                dataset::DatasetAction::Get { dataset: name } => {
                    output.print(&dataset::get(client, &name).await?)?;
                }
                dataset::DatasetAction::Create(args) => {
                    output.print(&dataset::create(client, &args).await?)?;
                }
                dataset::DatasetAction::Update(args) => {
                    output.print(&dataset::update(client, &args).await?)?;
                }
                dataset::DatasetAction::Delete(args) => {
                    output.print(&dataset::delete(client, &args, output).await?)?;
                }
            }

            Ok(())
        }

        Some(Commands::Upload(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            output.print(&upload::run(&client, &args, output).await?)?;

            Ok(())
        }

        Some(Commands::Delete(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            output.print(&delete::run(&client, &args, output).await?)?;

            Ok(())
        }

        Some(Commands::List(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            let stream = list::run(&client, &args).await?;

            match output.format {
                OutputFormat::Json => {
                    tokio::pin!(stream);
                    while let Some(entry) = stream.next().await {
                        if let Err(err) = output.print_json_line(&list::ListEntry::from(entry?)) {
                            if is_broken_pipe(&err) {
                                break;
                            }
                            return Err(err);
                        }
                    }
                }
                OutputFormat::Text => {
                    let entries = stream
                        .map(|entry| entry.map(list::ListEntry::from))
                        .try_collect()
                        .await?;
                    output.print(&list::ListResult { entries })?;
                }
            }
            Ok(())
        }

        Some(Commands::Import(args)) => {
            // Resolved lazily: --dry-run neither authenticates nor writes.
            let connect = || {
                let api_key = get_api_key(cli.api_key, &config)?;
                let region = cli.region.ok_or_else(|| {
                    topk::import::Error::InvalidArgument(
                        "--region is required to import (or set TOPK_REGION). \
                         List available regions at https://docs.topk.io/regions"
                            .to_string(),
                    )
                })?;
                Ok(make_client(&api_key, &region, &cli.host, cli.https))
            };
            topk::commands::import::run(connect, &args, output).await?;
            Ok(())
        }

        Some(Commands::Ask(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            let result = ask::run(&client, &args, output).await?;
            let paths = match args.output_dir.as_deref() {
                Some(dir) => search::save_search_results(dir, &result.refs)?,
                None => HashMap::default(),
            };

            match output.format {
                OutputFormat::Text => {
                    output.print(&result)?;

                    if !result.facts.is_empty() {
                        output.meta(&format!(
                            "{} {}",
                            "Confidence:".dimmed(),
                            format!("{:.2}%", result.confidence).dimmed().bold()
                        ));
                    }

                    if let Some(refs_text) = result.render_refs(&paths) {
                        output.print(&refs_text)?;
                    }

                    if let Some(dir) = &args.output_dir {
                        if !result.refs.is_empty() {
                            output.success(&format!("References saved to '{}'.", dir.display()));
                        }
                    }
                }
                OutputFormat::Json => {
                    output.print_json(&result)?;
                }
            }

            Ok(())
        }

        Some(Commands::Search(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            let result = search::run(&client, &args).await?;

            let paths = match args.output_dir.as_deref() {
                Some(dir) => {
                    let refs = result
                        .results
                        .iter()
                        .enumerate()
                        .map(|(i, r)| ((i + 1).to_string(), r.clone()))
                        .collect();

                    search::save_search_results(dir, &refs)?
                }
                None => HashMap::default(),
            };

            match output.format {
                OutputFormat::Text => {
                    output.print(&result.render(&paths))?;

                    if let Some(dir) = &args.output_dir {
                        if !result.results.is_empty() {
                            output.success(&format!("References saved to '{}'.", dir.display()));
                        }
                    }
                }
                OutputFormat::Json => {
                    output.print_json(&result)?;
                }
            }

            Ok(())
        }

        Some(Commands::Logout) => {
            config::clear()?;
            dataset_region_cache::clear()?;
            output.success("Logged out.");
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

/// Gets the API key from the CLI arguments or the config file.
fn get_api_key(api_key: Option<String>, config: &config::Config) -> Result<String, Error> {
    if let Some(key) = api_key {
        return Ok(key);
    }

    if let Some(key) = config.api_key.clone() {
        return Ok(key);
    }

    Err(Error::Unauthenticated(format!(
        "API key not set. Run `topk login` or set TOPK_API_KEY environment variable."
    )))
}
