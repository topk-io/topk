use std::path::PathBuf;

use topk::client::{make_client, make_global_client};
use topk::commands;
use topk::datasets::{ensure_unique_region, get_region, make_cached_datasets_client};
use topk::output::{Output, OutputFormat};
use topk::util::resolve_query;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use topk_rs::proto::v1::ctx::doc::DocId;

use commands::{ask, auth, dataset, delete, list, search, upload};

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
    #[arg(short = 'o', long, default_value = "human-readable", global = true)]
    output: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with TopK (manage your API key)
    Auth {
        #[command(subcommand)]
        action: Option<auth::AuthAction>,
    },

    /// Get a grounded answer from documents with source citations for a query
    Ask {
        /// Question to ask (reads from stdin if omitted)
        query: Option<String>,
        /// Dataset to search (repeatable)
        #[arg(short = 'd', long = "dataset")]
        datasets: Vec<String>,
        /// Query mode
        #[arg(short = 'm', long)]
        mode: Option<ask::Mode>,
        /// Metadata fields to include in results (repeatable)
        #[arg(short = 'f', long = "field")]
        fields: Option<Vec<String>>,
        /// Save search result content (images, text chunks) to a directory
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,
    },

    /// Find relevant passages in documents for a query
    Search {
        /// Search query (reads from stdin if omitted)
        query: Option<String>,
        /// Dataset to search (repeatable)
        #[arg(short = 'd', long = "dataset")]
        datasets: Vec<String>,
        /// Number of results to return
        #[arg(short = 'k', long, default_value = "10")]
        top_k: u32,
        /// Metadata fields to include in results (repeatable)
        #[arg(short = 'f', long = "field")]
        fields: Option<Vec<String>>,
        /// Save search results content (images, text chunks) to a directory
        #[arg(long, value_name = "DIR", num_args = 0..=1, default_missing_value = ".")]
        output_dir: Option<PathBuf>,
    },

    /// Upload files
    Upload {
        /// Dataset to upload into
        #[arg(short = 'd', long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Recurse into subdirectories when PATTERN is a directory
        #[arg(short = 'r', long)]
        recursive: bool,
        /// Number of concurrent uploads (1–64)
        #[arg(short = 'c', long, default_value = "32", value_parser = clap::value_parser!(u64).range(1..=64))]
        concurrency: u64,
        /// Skip upload confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
        /// Preview files without uploading
        #[arg(long)]
        dry_run: bool,
        /// Wait for all uploaded files to be fully processed
        #[arg(short = 'w', long)]
        wait: bool,
        /// Timeout for uploading files in seconds (default: 30 minutes)
        #[arg(long, value_name = "SECS", default_value = "1800")]
        timeout: u64,
        /// File path, directory, or glob pattern (e.g. "./report.pdf" "./docs" "*.pdf" "docs/**/*.md")
        #[arg(value_name = "PATTERN")]
        pattern: String,
    },

    /// Delete a document
    Delete {
        /// Dataset name
        #[arg(short = 'd', long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Document ID
        #[arg(long)]
        id: DocId,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// List documents in a dataset
    List {
        /// Dataset to list documents from
        #[arg(short = 'd', long, value_name = "DATASET_NAME")]
        dataset: String,
        /// Metadata fields to include (repeatable)
        #[arg(short = 'f', long = "field")]
        fields: Option<Vec<String>>,
    },

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
async fn main() -> std::process::ExitCode {
    let cli = Cli::parse();

    let output = Output::new(cli.output);

    if cli.command.is_none() {
        Cli::command().print_help().unwrap();
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

    let host = cli.host.unwrap_or_else(|| "topk.io".to_string());
    let https = cli.https;

    match command {
        Commands::Completions { shell } => {
            generate(shell, &mut Cli::command(), "topk", &mut std::io::stdout());
        }

        Commands::Auth { action } => match action.unwrap_or(auth::AuthAction::Login) {
            auth::AuthAction::Login => {
                auth::resolve(cli.api_key, &host, https, true)?;
            }
            auth::AuthAction::Logout => {
                output.print(&auth::logout()?)?;
            }
        },

        Commands::Dataset { action } => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            match action {
                dataset::DatasetAction::List => {
                    let result = dataset::list(&mut client).await?;
                    output.print(&result)?;
                }
                dataset::DatasetAction::Get { dataset: name } => {
                    output.print(&dataset::get(&mut client, &name).await?)?;
                }
                dataset::DatasetAction::Create {
                    dataset: name,
                    region,
                } => {
                    let result = dataset::create(&mut client, &name, &region).await?;
                    output.print(&result)?;
                }
                dataset::DatasetAction::Delete { dataset: name, yes } => {
                    let result = dataset::delete(&mut client, &name, yes, output).await?;
                    output.print(&result)?;
                }
            }
        }

        Commands::Upload {
            dataset,
            recursive,
            concurrency,
            yes,
            dry_run,
            wait,
            timeout,
            pattern,
        } => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            let region = get_region(&mut datasets_client, &dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(
                &upload::run(
                    &client,
                    &dataset,
                    &pattern,
                    recursive,
                    concurrency as usize,
                    yes,
                    dry_run,
                    wait,
                    Some(std::time::Duration::from_secs(timeout)),
                    output,
                )
                .await?,
            )?;
        }

        Commands::Delete { dataset, id, yes } => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            let region = get_region(&mut datasets_client, &dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&delete::run(&client, &dataset, id, yes, output).await?)?;
        }

        Commands::List { dataset, fields } => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            let region = get_region(&mut datasets_client, &dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            list::run(&client, &dataset, fields, output).await?;
        }

        Commands::Ask {
            query,
            datasets,
            mode,
            fields,
            output_dir,
        } => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };
            let query = resolve_query(query)?.ok_or_else(|| {
                anyhow::anyhow!("query is required; pass it as an argument or pipe it via stdin")
            })?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = ensure_unique_region(&mut datasets_client, datasets.clone()).await?;
            let client = make_client(&api_key, &region, &host, https);

            ask::run(&client, query, datasets, mode, fields, output_dir, output).await?;
        }

        Commands::Search {
            query,
            datasets,
            top_k,
            fields,
            output_dir,
        } => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };
            let query = resolve_query(query)?.ok_or_else(|| {
                anyhow::anyhow!("query is required; pass it as an argument or pipe it via stdin")
            })?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));
            let region = ensure_unique_region(&mut datasets_client, datasets.clone()).await?;
            let client = make_client(&api_key, &region, &host, https);

            search::run(&client, query, datasets, top_k, fields, output_dir, output).await?;
        }
    }

    Ok(())
}
