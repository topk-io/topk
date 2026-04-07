use std::io::{IsTerminal, Read};

use topk::commands;
use topk::output::{Output, OutputArg};
use tracing_subscriber::EnvFilter;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use topk_rs::{proto::v1::ctx::doc::DocId, Client, ClientConfig};

use commands::{ask, dataset, delete, search, upload, upsert};
use topk::util::parse_kv;

#[derive(Parser)]
#[command(name = "topk", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// TopK API key (overrides TOPK_API_KEY environment variable)
    #[arg(long, env = "TOPK_API_KEY", global = true, hide_env_values = true)]
    api_key: Option<String>,

    /// TopK Region (overrides TOPK_REGION environment variable, available regions: https://docs.topk.io/regions)
    #[arg(long, env = "TOPK_REGION", global = true)]
    region: Option<String>,

    /// Host (overrides TOPK_HOST environment variable, default: topk.io)
    #[arg(long, env = "TOPK_HOST", global = true, hide = true)]
    host: Option<String>,

    /// Output mode: human for interactive terminal use, agent for machine-readable JSON
    #[arg(long, default_value = "human", global = true)]
    output: OutputArg,

    /// Shorthand for --output agent
    #[arg(long, alias = "json", global = true)]
    agent: bool,

    /// Pretty-print JSON output (only applies in agent mode)
    #[arg(long, global = true)]
    pretty: bool,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a grounded answer from documents with source citations for a query
    Ask {
        /// Question to ask (reads from stdin if omitted)
        query: Option<String>,
        /// Dataset(s) to search (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Response mode
        #[arg(long, default_value = "auto")]
        mode: ask::Mode,
        /// Metadata fields to include in results
        #[arg(long = "field", value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Find relevant passages in documents for a query
    Search {
        /// Search query (reads from stdin if omitted)
        query: Option<String>,
        /// Dataset(s) to search (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Number of results to return
        #[arg(long, default_value = "10")]
        top_k: u32,
        /// Metadata fields to include in results
        #[arg(long = "field", value_delimiter = ',')]
        fields: Vec<String>,
    },

    /// Upload files or directories
    Upload {
        /// Path(s) to files or directories, comma-separated
        #[arg(value_name = "PATHS", value_delimiter = ',')]
        paths: Vec<std::path::PathBuf>,
        /// Dataset to upload into
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Scan directory recursively
        #[arg(short = 'r')]
        recursive: bool,
        /// Number of concurrent uploads (default: 4)
        #[arg(short = 'c', long, default_value = "4")]
        concurrency: usize,
        /// Create the dataset without prompting if it does not exist
        #[arg(short = 'y')]
        yes: bool,
        /// Preview files without uploading
        #[arg(long)]
        dry_run: bool,
        /// Wait for all files to be uploaded and fully processed
        #[arg(long)]
        wait: bool,
    },

    /// Upsert a document
    Upsert {
        /// Dataset name
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Document ID
        #[arg(long)]
        document_id: DocId,
        /// Path to file
        path: std::path::PathBuf,
        /// Metadata key=value pairs
        #[arg(long = "meta", value_parser = parse_kv)]
        metadata: Vec<(String, String)>,
        /// Block until the document is uploaded and fully processed
        #[arg(long)]
        wait: bool,
    },

    /// Delete a document
    Delete {
        /// Dataset name
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Document ID
        #[arg(long)]
        document_id: DocId,
        /// Skip confirmation prompt
        #[arg(short = 'y')]
        yes: bool,
    },

    /// Manage datasets (create, list, delete)
    Dataset {
        #[command(subcommand)]
        action: dataset::DatasetAction,
    },
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("topk=debug"))
            .with_target(false)
            .init();
    }

    let output = Output::new(cli.agent, cli.output, cli.pretty);

    if cli.command.is_none() {
        print_welcome();
        Cli::command().print_help().unwrap();
        println!();
        return std::process::ExitCode::SUCCESS;
    }

    if let Err(e) = run(cli, &output).await {
        output.error(&e);
        return std::process::ExitCode::FAILURE;
    }

    std::process::ExitCode::SUCCESS
}

async fn run(cli: Cli, output: &Output) -> Result<()> {
    let client = make_client(cli.api_key, cli.region, cli.host)?;

    match cli.command.expect("checked above") {
        Commands::Dataset { action } => match action {
            dataset::DatasetAction::List => {
                output.print(&dataset::list(&client).await?)?;
            }
            dataset::DatasetAction::Get { dataset: name } => {
                output.print(&dataset::get(&client, &name).await?)?;
            }
            dataset::DatasetAction::Create { dataset: name } => {
                output.print(&dataset::create(&client, &name).await?)?;
            }
            dataset::DatasetAction::Delete { dataset: name, yes } => {
                output.print(&dataset::delete(&client, &name, yes, &output).await?)?;
            }
        },

        Commands::Upload {
            paths,
            dataset,
            recursive,
            concurrency,
            yes,
            dry_run,
            wait,
        } => {
            output.print(
                &upload::run(
                    &client,
                    &dataset,
                    &paths,
                    recursive,
                    concurrency,
                    yes,
                    dry_run,
                    wait,
                    output,
                )
                .await?,
            )?;
        }

        Commands::Upsert {
            dataset,
            document_id,
            path,
            metadata,
            wait,
        } => {
            let mut result = upsert::run(&client, &dataset, document_id, path, metadata).await?;
            if wait {
                output.progress("Processing...");
                client
                    .dataset(&dataset)
                    .wait_for_handle(&result.handle, None)
                    .await?;
                result.processed = true;
            }
            output.print(&result)?;
        }

        Commands::Delete {
            dataset,
            document_id,
            yes,
        } => {
            output.print(&delete::run(&client, &dataset, document_id, yes, &output).await?)?;
        }

        Commands::Ask {
            query,
            sources,
            mode: cmd_mode,
            fields,
        } => {
            let query = match resolve_query(query)? {
                Some(q) => q,
                None => {
                    Cli::command()
                        .find_subcommand_mut("ask")
                        .expect("ask subcommand")
                        .print_help()?;
                    return Ok(());
                }
            };
            output.print(
                &ask::run(
                    &client,
                    query,
                    sources,
                    Some(cmd_mode.into()),
                    fields,
                    output,
                )
                .await?,
            )?;
        }

        Commands::Search {
            query,
            sources,
            top_k,
            fields,
        } => {
            let query = match resolve_query(query)? {
                Some(q) => q,
                None => {
                    Cli::command()
                        .find_subcommand_mut("search")
                        .expect("search subcommand")
                        .print_help()?;
                    return Ok(());
                }
            };
            output.print(&search::run(&client, query, sources, top_k, fields).await?)?;
        }
    }

    Ok(())
}

fn resolve_query(query: Option<String>) -> Result<Option<String>> {
    if let Some(q) = query {
        return Ok(Some(q));
    }
    if std::io::stdin().is_terminal() {
        return Ok(None);
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let q = buf.trim().to_string();
    if q.is_empty() {
        Ok(None)
    } else {
        Ok(Some(q))
    }
}

fn print_welcome() {
    const BOLD: &str = "\x1b[1m";
    const CYAN: &str = "\x1b[36m";
    const ORANGE: &str = "\x1b[33m";
    const RED: &str = "\x1b[31m";
    const DIM: &str = "\x1b[2m";
    const RESET: &str = "\x1b[0m";

    println!();
    println!("{}Welcome to TopK CLI{}", BOLD, RESET);
    println!("{}Turn raw files into searchable knowledge.{}", DIM, RESET);
    println!();

    let api_key = std::env::var("TOPK_API_KEY").ok().filter(|v| !v.is_empty());
    let region = std::env::var("TOPK_REGION").ok().filter(|v| !v.is_empty());

    let api_key_status = match &api_key {
        Some(key) => format!("{DIM}{}{RESET}", "*".repeat(key.chars().count())),
        None => format!(
            "{RED}✗{RESET} {DIM}set TOPK_API_KEY environment variable or pass --api-key TOPK_API_KEY. Create your API key: https://console.topk.io{RESET}"
        ),
    };
    let region_status = match &region {
        Some(r) => format!("{ORANGE}{r}{RESET}"),
        None => format!(
            "{RED}✗{RESET} {DIM}set TOPK_REGION environment variable or pass --region TOPK_REGION. List available regions: https://docs.topk.io/regions{RESET}"
        ),
    };

    println!("{BOLD}Configuration:{RESET}");
    println!("{CYAN}API Key:{RESET}  {api_key_status}");
    println!("{CYAN}Region:{RESET}   {region_status}");
    println!();
}

fn make_client(
    api_key: Option<String>,
    region: Option<String>,
    host: Option<String>,
) -> Result<Client> {
    let api_key = api_key.ok_or_else(|| {
        anyhow::anyhow!(
            "API key not set. Set TOPK_API_KEY environment variable or pass --api-key TOPK_API_KEY. Create your API key: https://console.topk.io"
        )
    })?;
    let region = region.ok_or_else(|| {
        anyhow::anyhow!(
            "Region not set. Set TOPK_REGION environment variable or pass --region TOPK_REGION. List available regions: https://docs.topk.io/regions"
        )
    })?;
    let host = host.unwrap_or_else(|| "topk.io".to_string());
    let https = std::env::var("TOPK_HTTPS")
        .map(|v| v == "true")
        .unwrap_or(true);

    Ok(Client::new(
        ClientConfig::new(api_key, region)
            .with_host(host)
            .with_https(https),
    ))
}
