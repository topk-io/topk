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

    /// Output machine-readable JSON
    #[arg(long, visible_alias = "agent", global = true)]
    json: bool,

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
        /// Dataset(s) to search, comma-separated; if omitted, searches all datasets
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Response mode
        #[arg(long, default_value = "auto")]
        mode: ask::Mode,
        /// Metadata fields to include in results, comma-separated
        #[arg(long = "fields", value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Find relevant passages in documents for a query
    Search {
        /// Search query (reads from stdin if omitted)
        query: Option<String>,
        /// Dataset(s) to search, comma-separated; if omitted, searches all datasets
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Number of results to return
        #[arg(long, default_value = "10")]
        top_k: u32,
        /// Metadata fields to include in results, comma-separated
        #[arg(long = "fields", value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Upload files matching regex patterns from the current directory
    Upload {
        /// Regex pattern(s) matched against file paths relative to the current directory
        #[arg(value_name = "PATTERNS")]
        patterns: Vec<String>,
        /// Dataset to upload into
        #[arg(short = 'd', long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Number of concurrent uploads (1–64)
        #[arg(short = 'c', long, default_value = "32", value_parser = clap::value_parser!(u64).range(1..=64))]
        concurrency: u64,
        /// Create the dataset without prompting if it does not exist
        #[arg(short = 'y')]
        yes: bool,
        /// Preview files without uploading
        #[arg(long)]
        dry_run: bool,
        /// Wait for all files to be fully processed (default in interactive mode)
        #[arg(long, conflicts_with = "no_wait")]
        wait: bool,
        /// Skip waiting for processing
        #[arg(long, conflicts_with = "wait")]
        no_wait: bool,
    },

    /// Upsert a document
    Upsert {
        /// Dataset name
        #[arg(short = 'd', long, value_name = "DATASET_NAME")]
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
        /// Preview the upsert without uploading
        #[arg(long)]
        dry_run: bool,
    },

    /// Delete a document
    Delete {
        /// Dataset name
        #[arg(short = 'd', long, value_name = "DATASET_NAME")]
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

    let output = Output::new(cli.json, cli.output, cli.pretty);

    if cli.command.is_none() {
        print_welcome(cli.api_key.as_deref(), cli.region.as_deref());
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
    let command = cli.command.expect("checked above");

    // Show subcommand help before requiring credentials.
    match &command {
        Commands::Upload { patterns, .. } if patterns.is_empty() => {
            return print_subcommand_help("upload");
        }
        Commands::Ask { query, .. } if query.is_none() && std::io::stdin().is_terminal() => {
            return print_subcommand_help("ask");
        }
        Commands::Search { query, .. } if query.is_none() && std::io::stdin().is_terminal() => {
            return print_subcommand_help("search");
        }
        _ => {}
    }

    let client = make_client(cli.api_key, cli.region, cli.host)?;

    match command {
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
            patterns,
            dataset,
            concurrency,
            yes,
            dry_run,
            wait,
            no_wait,
        } => {
            output.print(
                &upload::run(
                    &client,
                    &dataset,
                    &patterns,
                    concurrency as usize,
                    yes,
                    dry_run,
                    wait,
                    no_wait,
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
            dry_run,
        } => {
            let mut result =
                upsert::run(&client, &dataset, document_id, path, metadata, dry_run).await?;
            if wait && !dry_run {
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
            let query = resolve_query(query)?
                .ok_or_else(|| anyhow::anyhow!("query is required; pass it as an argument or pipe it via stdin"))?;
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
            let query = resolve_query(query)?
                .ok_or_else(|| anyhow::anyhow!("query is required; pass it as an argument or pipe it via stdin"))?;
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

fn print_welcome(api_key: Option<&str>, region: Option<&str>) {
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

    let api_key_status = match api_key {
        Some(key) => format!("{DIM}{}{RESET}", "*".repeat(key.chars().count())),
        None => format!(
            "{RED}✗{RESET} {DIM}set TOPK_API_KEY environment variable or pass --api-key TOPK_API_KEY. Create your API key: https://console.topk.io{RESET}"
        ),
    };
    let region_status = match region {
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

fn print_subcommand_help(name: &str) -> Result<()> {
    Cli::command()
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("subcommand '{name}' not found"))
        .print_help()?;
    println!();
    Ok(())
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
