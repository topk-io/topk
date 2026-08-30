use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use colored::Colorize;

use topk::commands::login;
use topk::config;
use topk::endpoint::Endpoint;

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

    /// Bulk import from a database, file or object store
    #[cfg(feature = "import")]
    Import(topk::commands::import::ImportArgs),

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

fn main() -> ExitCode {
    // Rust ignores SIGPIPE, so `topk … | head` panics on the closed pipe.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_DFL) };
    // AWS SSO for duckdb: sets AWS_CONFIG_FILE, UB once runtime threads getenv.
    #[cfg(feature = "import")]
    if std::env::args().any(|a| a == "import") {
        topk::import::source::aws_process_profile();
    }
    async_main()
}

#[tokio::main]
async fn async_main() -> ExitCode {
    let cli = Cli::parse();
    init_logging(cli.verbose);
    match run(&cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {e}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    match &cli.command {
        Some(Commands::Login) => {
            let api_key = match cli.endpoint.api_key()? {
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

            Ok(ExitCode::SUCCESS)
        }

        #[cfg(feature = "import")]
        Some(Commands::Import(args)) => {
            topk::commands::import::run(&cli.endpoint, args, cli.output == Output::Json).await
        }

        Some(Commands::Logout) => {
            config::clear()?;
            eprintln!("{} Logged out.", "✓".green());
            Ok(ExitCode::SUCCESS)
        }

        Some(Commands::Completions { shell }) => {
            generate(*shell, &mut Cli::command(), "topk", &mut std::io::stdout());
            Ok(ExitCode::SUCCESS)
        }

        None => {
            Cli::command().print_help()?;
            Ok(ExitCode::SUCCESS)
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
