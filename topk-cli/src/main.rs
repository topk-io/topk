use topk::commands;
use topk::output::{Output, OutputArg};
use tracing_subscriber::EnvFilter;

use anyhow::Result;
use clap::{Parser, Subcommand};
use topk_rs::{proto::v1::ctx::doc::DocId, Client, ClientConfig};

use topk::util::parse_kv;
use commands::{ask, dataset, delete, search, upload, upsert};

#[derive(Parser)]
#[command(
    name = "topk",
    about = "Ingest files into datasets, then search or ask questions with cited answers sourced from your content.",
    version,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// API key (overrides TOPK_API_KEY)
    #[arg(long, env = "TOPK_API_KEY", global = true, hide_env_values = true, hide = true)]
    api_key: Option<String>,

    /// Region (overrides TOPK_REGION)
    #[arg(long, env = "TOPK_REGION", global = true, hide = true)]
    region: Option<String>,

    /// Host (overrides TOPK_HOST, default: topk.io)
    #[arg(long, env = "TOPK_HOST", global = true, hide = true)]
    host: Option<String>,

    /// Output mode
    #[arg(long, default_value = "human", global = true)]
    output: OutputArg,

    /// Output as JSON for agent/machine consumption (shorthand for --output agent)
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
    /// Manage datasets
    Dataset {
        #[command(subcommand)]
        action: dataset::DatasetAction,
    },

    /// Upload a file or directory of supported files to a dataset (creates dataset if needed)
    Upload {
        /// Path to a file or directory
        #[arg(value_name = "PATH_TO_DIR_OR_FILE")]
        path: std::path::PathBuf,
        /// Dataset to upload into
        #[arg(long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Scan directory recursively
        #[arg(short = 'r')]
        recursive: bool,
        /// Number of concurrent uploads (default: 4)
        #[arg(short = 'c', long, default_value = "4")]
        concurrency: usize,
        /// Preview files without uploading
        #[arg(long)]
        dry_run: bool,
        /// Wait for all files to be fully processed after uploading
        #[arg(long)]
        wait: bool,
    },

    /// Upload or replace a single file in a dataset
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

    /// Delete a document from a dataset
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

    /// Ask a question across one or more datasets
    Ask {
        /// Question to ask (reads from stdin if omitted)
        query: Option<String>,
        /// Dataset(s) to search (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        sources: Vec<String>,
        /// Response mode
        #[arg(long, default_value = "auto")]
        mode: ask::AskMode,
        /// Metadata fields to include in results
        #[arg(long = "field", value_delimiter = ',')]
        fields: Option<Vec<String>>,
    },

    /// Search across one or more datasets
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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("topk=debug"))
            .with_target(false)
            .init();
    }

    let output = Output::new(cli.agent, cli.output, cli.pretty);

    if let Err(e) = run(cli, &output).await {
        output.error(&e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli, output: &Output) -> Result<()> {
    let client = make_client(cli.api_key, cli.region, cli.host)?;

    match cli.command {
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
                output.print(&dataset::delete(&client, &name, yes).await?)?;
            }
        },

        Commands::Upload { path, dataset, recursive, concurrency, dry_run, wait } => {
            let (result, errors) = upload::run(&client, &dataset, &path, recursive, concurrency, dry_run, wait).await?;
            for e in &errors {
                output.error(&anyhow::anyhow!("{}: {}", e.path, e.error));
            }
            output.print(&result)?;
        }

        Commands::Upsert { dataset, document_id, path, metadata, wait } => {
            let mut result = upsert::run(&client, &dataset, document_id, path, metadata).await?;
            if wait {
                output.progress("Processing...");
                client.dataset(&dataset).wait_for_handle(&result.handle, None).await?;
                result.processed = true;
            }
            output.print(&result)?;
        }

        Commands::Delete { dataset, document_id, yes } => {
            output.print(&delete::run(&client, &dataset, document_id, yes).await?)?;
        }

        Commands::Ask { query, sources, mode: cmd_mode, fields } => {
            output.print(&ask::run(&client, resolve_query(query)?, sources, Some(cmd_mode.into()), fields, output).await?)?;
        }

        Commands::Search { query, sources, top_k, fields } => {
            output.print(&search::run(&client, resolve_query(query)?, sources, top_k, fields).await?)?;
        }
    }

    Ok(())
}

fn resolve_query(query: Option<String>) -> Result<String> {
    if let Some(q) = query {
        return Ok(q);
    }
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let q = buf.trim().to_string();
    if q.is_empty() {
        anyhow::bail!("no query provided");
    }
    Ok(q)
}

fn make_client(api_key: Option<String>, region: Option<String>, host: Option<String>) -> Result<Client> {
    let api_key = api_key.ok_or_else(|| anyhow::anyhow!("TOPK_API_KEY env variable is not set. Create an API key at https://console.topk.io/"))?;
    let region = region.ok_or_else(|| anyhow::anyhow!("TOPK_REGION env variable is not set. List available regions at https://docs.topk.io/regions"))?;
    let host = host.unwrap_or_else(|| "topk.io".to_string());
    let https = std::env::var("TOPK_HTTPS").map(|v| v == "true").unwrap_or(true);

    Ok(Client::new(
        ClientConfig::new(api_key, region)
            .with_host(host)
            .with_https(https),
    ))
}
