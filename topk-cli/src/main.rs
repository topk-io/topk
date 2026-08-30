use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use colored::Colorize;

use topk::commands::login;
use topk::config;
use topk::endpoint::Endpoint;
use topk_rs::Error;

#[derive(Parser)]
#[command(name = "topk", version, after_help = agent_mode().then(|| include_str!("../README.md")))]
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

    #[command(flatten)]
    endpoint: Endpoint,

    /// Output format
    #[arg(
        short = 'o',
        long,
        default_value = "text",
        global = true,
        help_heading = "Global options"
    )]
    output: Output,
}

/// `json` puts results on stdout as JSON; stderr chatter is the same either way.
#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Output {
    Text,
    Json,
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

/// For an agent `--help` is all it will ever know about the tool.
fn agent_mode() -> bool {
    ["CLAUDECODE", "AGENT"]
        .iter()
        .any(|v| std::env::var_os(v).is_some_and(|s| !s.is_empty()))
        || std::env::args().any(|a| a == "--agent")
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(cli.verbose);
    match run(&cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e:#}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: &Cli) -> Result<(), Error> {
    match &cli.command {
        Some(Commands::Login) => {
            let api_key = match cli.endpoint.api_key() {
                Some(key) => Some(key),
                None => login::run(&cli.endpoint)?,
            };

            match api_key {
                Some(api_key) => {
                    config::set_api_key(api_key)?;
                    eprintln!("{} API key saved.", "✓".green());
                }
                None => {
                    println!("Skipping authentication.");
                }
            }

            Ok(())
        }

        Some(Commands::Logout) => {
            config::clear()?;
            eprintln!("{} Logged out.", "✓".green());
            Ok(())
        }

        Some(Commands::Completions { shell }) => {
            generate(*shell, &mut Cli::command(), "topk", &mut std::io::stdout());
            Ok(())
        }

        None => {
            Cli::command().print_help()?;
            Ok(())
        }
    }
}

/// Off unless `-v` or `RUST_LOG` (which wins); stdout carries results.
fn init_logging(verbose: bool) {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| match verbose {
            true => tracing_subscriber::EnvFilter::new("info"),
            false => tracing_subscriber::EnvFilter::new("off"),
        });
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}
