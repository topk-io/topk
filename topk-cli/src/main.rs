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
    let config = config::load();

    match cli.command {
        Some(Commands::Login) => {
            let api_key = login::run(&cli.host, cli.https)?;

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
                        output
                            .print_json_line(&list::ListEntry::from(entry?))
                            .map_err(|e| Error::Internal(e.to_string()))?;
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

        Some(Commands::Ask(args)) => {
            let api_key = get_api_key(cli.api_key, &config)?;

            let mut datasets_client =
                make_cached_datasets_client(make_global_client(&api_key, &cli.host, cli.https));

            let region = ensure_unique_region(&mut datasets_client, args.datasets.clone()).await?;
            let client = make_client(&api_key, &region, &cli.host, cli.https);

            let result = ask::run(&client, &args, output).await?;

            match output.format {
                OutputFormat::Text => {
                    // Print the answer
                    output.print(&result)?;

                    let output_dir = match &args.output_dir {
                        Some(dir) => Some(dir.clone()),
                        None if !result.refs.is_empty() => output.prompt_dir(
                            "\nSave document references to directory (or press Enter to skip)",
                        )?,
                        None => None,
                    };

                    let refs_text = result
                        .refs
                        .iter()
                        .map(|(ref_id, result)| {
                            let path = output_dir
                                .as_ref()
                                .map(|dir| search::write_search_result(dir, ref_id, result))
                                .transpose()?;

                            Ok::<_, Error>(search::render_search_result(
                                ref_id,
                                result,
                                path.as_deref(),
                                Some(560),
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .join("\n\n");

                    output.print(&format!("{}\n{refs_text}", "References:".bold()))?;

                    if output_dir.is_some() {
                        output.success("References saved.");
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

            match output.format {
                OutputFormat::Text => {
                    let output_dir = match &args.output_dir {
                        Some(dir) => Some(dir.clone()),
                        None => output
                            .prompt_dir("Save results to a directory (or press Enter to skip)")?,
                    };

                    let rendered_text = result
                        .iter()
                        .enumerate()
                        .map(|(i, result)| {
                            let ref_id = (i + 1).to_string();
                            let path = output_dir
                                .as_ref()
                                .map(|dir| search::write_search_result(dir, &ref_id, result))
                                .transpose()?;

                            Ok::<_, Error>(search::render_search_result(
                                &ref_id,
                                result,
                                path.as_deref(),
                                None,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?
                        .join("\n\n");

                    output.print(&rendered_text)?;

                    if output_dir.is_some() {
                        output.success("References saved.");
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

    Err(Error::Input(anyhow::anyhow!(
        "API key not set. Run `topk login` or set TOPK_API_KEY environment variable."
    )))
}
