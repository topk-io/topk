use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use topk_rs::proto::v1::ctx::doc::DocId;

use topk::client::{make_client, make_global_client};
use topk::commands::{ask, auth, dataset, delete, list, search, upload};
use topk::datasets::{ensure_unique_region, get_region, make_cached_datasets_client};
use topk::output::{Output, OutputFormat};
use topk::util::resolve_query;

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
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,
    },

    /// Upload files
    Upload(upload::UploadArgs),

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
    let host = cli.host.unwrap_or_else(|| "topk.io".to_string());
    let https = cli.https;

    match cli.command {
        Some(Commands::Completions { shell }) => {
            generate(shell, &mut Cli::command(), "topk", &mut std::io::stdout());
        }

        Some(Commands::Auth { action }) => match action.unwrap_or(auth::AuthAction::Login) {
            auth::AuthAction::Login => {
                auth::resolve(cli.api_key, &host, https, true)?;
            }
            auth::AuthAction::Logout => {
                output.print(&auth::logout()?)?;
            }
        },

        Some(Commands::Dataset { action }) => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let client = make_cached_datasets_client(make_global_client(&api_key, &host, https));

            match action {
                dataset::DatasetAction::List => {
                    let result = dataset::list(client).await?;
                    output.print(&result)?;
                }
                dataset::DatasetAction::Get { dataset: name } => {
                    output.print(&dataset::get(client, &name).await?)?;
                }
                dataset::DatasetAction::Create {
                    dataset: name,
                    region,
                } => {
                    output.print(&dataset::create(client, &name, &region).await?)?;
                }
                dataset::DatasetAction::Delete { dataset: name, yes } => {
                    output.print(&dataset::delete(client, &name, yes, output).await?)?;
                }
            }
        }

        Some(Commands::Upload(args)) => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            let region = get_region(&mut datasets_client, &args.dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&upload::run(&client, &args, output).await?)?;
        }

        Some(Commands::Delete { dataset, id, yes }) => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            let region = get_region(&mut datasets_client, &dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            output.print(&delete::run(&client, &dataset, id, yes, output).await?)?;
        }

        Some(Commands::List { dataset, fields }) => {
            let Some(api_key) = auth::resolve(cli.api_key, &host, https, false)? else {
                return Ok(());
            };

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &host, https));

            let region = get_region(&mut datasets_client, &dataset).await?;
            let client = make_client(&api_key, &region, &host, https);

            list::run(&client, &dataset, fields, output).await?;
        }

        Some(Commands::Ask {
            query,
            datasets,
            mode,
            fields,
            output_dir,
        }) => {
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

        Some(Commands::Search {
            query,
            datasets,
            top_k,
            fields,
            output_dir,
        }) => {
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

        None => {
            Cli::command().print_help()?;
        }
    }

    Ok(())
}
