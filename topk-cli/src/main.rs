use std::collections::HashMap;
use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use futures::TryStreamExt;
use tokio_stream::StreamExt;

use topk::commands::{ask, dataset, delete, list, login, search, upload};
use topk::config;
use topk::dataset_region_cache;
use topk::datasets::{ensure_unique_region, get_region, make_cached_datasets_client};
use topk::output::{is_broken_pipe, Output, OutputFormat};
use topk_rs::client::retry::{BackoffConfig, RetryConfig};
use topk_rs::{Client, ClientConfig, Error};

#[derive(Parser)]
#[command(name = "topk", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Log what the run is doing to stderr (RUST_LOG overrides)
    #[arg(short, long, global = true, help_heading = "Global options")]
    verbose: bool,

    /// Agent-oriented output: --help includes the full manual
    /// (auto-detected for AI assistants)
    #[arg(long, global = true, help_heading = "Global options")]
    agent: bool,

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
    #[arg(
        long,
        env = "TOPK_REGION",
        global = true,
        help_heading = "Global options"
    )]
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
    #[cfg(feature = "import")]
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

/// Off unless `-v` or `RUST_LOG` (which wins); stdout carries results.
fn init_logging(verbose: bool) {
    let filter = match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(_) if verbose => tracing_subscriber::EnvFilter::new("info"),
        Err(_) => return,
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

/// The README from `## Commands` down, compiled in.
const MANUAL_START: &str = "<!-- manual:start -->";
const MANUAL_END: &str = "<!-- manual:end -->";

fn manual() -> &'static str {
    let readme = include_str!("../README.md");
    let (_, rest) = readme.split_once(MANUAL_START).unwrap_or(("", readme));
    rest.split_once(MANUAL_END)
        .map_or(rest, |(manual, _)| manual)
}

/// For an agent `--help` is all it will ever know about the tool.
fn agent_mode() -> bool {
    ["CLAUDECODE", "AGENT", "TOPK_AGENT"]
        .iter()
        .any(|v| std::env::var_os(v).is_some_and(|s| !s.is_empty()))
        || std::env::args().any(|a| a == "--agent")
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    if agent_mode() {
        cmd = cmd.after_help(manual());
    }
    let mut cli = <Cli as clap::FromArgMatches>::from_arg_matches(&cmd.get_matches())
        .expect("clap derive produces matching matches");
    init_logging(cli.verbose);
    // Set-but-empty env vars (`TOPK_REGION=`) read as unset.
    cli.api_key = cli.api_key.filter(|v| !v.is_empty());
    cli.region = cli.region.filter(|v| !v.is_empty());

    let output = Output::new(cli.output);
    let (host, https) = (cli.host.clone(), cli.https);

    match run(cli, &output).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            output.error(&e);
            if let Some(hint) = endpoint_hint(&e, &host, https) {
                output.meta(&hint);
            }
            ExitCode::FAILURE
        }
    }
}

/// A transport failure says nothing about the mistake behind it. The common one
/// is a stale `TOPK_HTTPS=false` from the emulator: cleartext HTTP/2 reaches a
/// TLS endpoint, which answers with a GOAWAY the h2 library reports as
/// `FRAME_SIZE_ERROR`.
fn endpoint_hint(e: &Error, host: &str, https: bool) -> Option<String> {
    let cleartext_at_tls =
        matches!(e, Error::Unexpected(message) if message.contains("h2 protocol error"));
    if cleartext_at_tls && !https {
        return Some(format!(
            "{host} answered as a TLS endpoint but the request went out in cleartext \
             — unset TOPK_HTTPS (or pass --https true)"
        ));
    }
    if matches!(e, Error::TransportError(_)) {
        return Some(match https {
            true => format!(
                "could not reach {host} over https — check --host and --region, \
                 and pass --https false for a plaintext endpoint such as the emulator"
            ),
            false => format!("could not reach {host} over http — check --host and --region"),
        });
    }
    None
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

            let client = make_cached_datasets_client(&api_key, &cli.host, cli.https);

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

            let client = client_for_dataset(&api_key, &cli.host, cli.https, &args.dataset).await?;

            output.print(&upload::run(&client, &args, output).await?)?;

            Ok(())
        }

        Some(Commands::Delete(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let client = client_for_dataset(&api_key, &cli.host, cli.https, &args.dataset).await?;

            output.print(&delete::run(&client, &args, output).await?)?;

            Ok(())
        }

        Some(Commands::List(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let client = client_for_dataset(&api_key, &cli.host, cli.https, &args.dataset).await?;

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

        #[cfg(feature = "import")]
        Some(Commands::Import(args)) => {
            // Lazy: --dry-run neither authenticates nor writes.
            let target = || {
                let api_key = get_api_key(cli.api_key.clone(), &config)?;
                let region = cli.region.clone().ok_or_else(|| {
                    topk::import::Error::InvalidArgument(
                        "--region is required to import (or set TOPK_REGION). \
                         List available regions at https://docs.topk.io/regions"
                            .to_string(),
                    )
                })?;
                Ok(Client::new(
                    ClientConfig::new(&api_key, &region)
                        .with_host(&cli.host)
                        .with_https(cli.https)
                        .with_retry_config(RetryConfig {
                            max_retries: 3,
                            backoff: BackoffConfig {
                                init_backoff: std::time::Duration::from_millis(250),
                                ..BackoffConfig::default()
                            },
                            ..RetryConfig::default()
                        }),
                ))
            };
            topk::commands::import::run(target, &args, output).await?;
            Ok(())
        }

        Some(Commands::Ask(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let client =
                client_for_datasets(&api_key, &cli.host, cli.https, args.datasets.clone()).await?;

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

            let client =
                client_for_datasets(&api_key, &cli.host, cli.https, args.datasets.clone()).await?;

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
async fn client_for_dataset(
    api_key: &str,
    host: &str,
    https: bool,
    dataset: &str,
) -> Result<Client, Error> {
    let mut datasets = make_cached_datasets_client(api_key, host, https);
    let region = get_region(&mut datasets, dataset).await?;

    Ok(Client::new(
        ClientConfig::new(api_key, region)
            .with_host(host)
            .with_https(https),
    ))
}

async fn client_for_datasets(
    api_key: &str,
    host: &str,
    https: bool,
    datasets: Vec<String>,
) -> Result<Client, Error> {
    let mut client = make_cached_datasets_client(api_key, host, https);
    let region = ensure_unique_region(&mut client, datasets).await?;

    Ok(Client::new(
        ClientConfig::new(api_key, region)
            .with_host(host)
            .with_https(https),
    ))
}

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
