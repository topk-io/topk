use std::process::ExitCode;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

use topk::commands::login;
use topk::config;
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
                    println!("Skipping authentication.");
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
