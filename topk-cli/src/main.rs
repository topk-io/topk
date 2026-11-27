use std::io::{ErrorKind, IsTerminal, Write};
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use tracing_indicatif::{
    filter::IndicatifFilter, style::ProgressStyle, IndicatifLayer, TickSettings,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

use topk::config::{Args as ConfigArgs, Config};
use topk::output::Output;
use topk::profile::SavedProfiles;
use topk::util::Interrupted;
use tracing::info;

#[derive(Parser)]
#[command(name = "topk", version, after_help = agent_mode().then(|| include_str!("../README.md")))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Log progress with -v, diagnostic details with -vv (RUST_LOG overrides)
    #[arg(short, long, action = clap::ArgAction::Count, global = true, help_heading = "Global options")]
    verbose: u8,

    /// Agent-oriented output: --help includes the full manual
    /// (auto-detected for AI assistants)
    #[arg(long, global = true, help_heading = "Global options")]
    agent: bool,

    /// Output format
    #[arg(
        short = 'o',
        long,
        default_value = "text",
        global = true,
        help_heading = "Global options"
    )]
    output: Output,

    #[command(flatten)]
    config: ConfigArgs,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage projects
    Project(topk::commands::project::Args),
    /// List available regions
    Region(topk::commands::region::Args),
    /// Log in with your TopK account in the browser
    Login(topk::commands::login::LoginArgs),
    /// Remove saved credentials
    Logout(topk::commands::logout::LogoutArgs),

    /// Switch between named profiles
    Profile(topk::commands::profile::Args),

    /// Run SQL against your collections
    Sql(topk::commands::sql::SqlArgs),

    /// Show usage of your organization, projects and collections
    Usage(topk::commands::usage::Args),

    /// Bulk import from a database, file or object store
    #[cfg(feature = "import")]
    Import(topk::commands::import::ImportArgs),

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
    let started = std::time::Instant::now();
    let result = run(&cli).await;
    info!(
        elapsed_ms = started.elapsed().as_millis(),
        success = result.as_ref().is_ok_and(|code| *code == ExitCode::SUCCESS),
        "command finished"
    );
    match result {
        Ok(code) => code,
        Err(e) if e.is::<Interrupted>() => ExitCode::from(130),
        // `topk … | head`: the reader went away; exit like a shell pipeline would.
        Err(e)
            if e.downcast_ref::<std::io::Error>().map(std::io::Error::kind)
                == Some(ErrorKind::BrokenPipe) =>
        {
            ExitCode::from(141)
        }
        Err(e) => {
            eprintln!("{} {e:#}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    let config = Config::load(&cli.config, SavedProfiles::new(Config::dir()?)).await?;
    match &cli.command {
        Some(Commands::Project(args)) => {
            topk::commands::project::run(config, args, cli.output, &mut std::io::stdout()).await
        }
        Some(Commands::Region(args)) => {
            topk::commands::region::run(config, args, cli.output, &mut std::io::stdout()).await
        }

        Some(Commands::Sql(args)) => {
            topk::commands::sql::run(config, args, cli.output, &mut std::io::stdout()).await
        }
        Some(Commands::Usage(args)) => {
            topk::commands::usage::run(config, args, cli.output, &mut std::io::stdout()).await
        }

        Some(Commands::Login(args)) => topk::commands::login::run(config, args).await,
        Some(Commands::Logout(args)) => topk::commands::logout::run(config, args).await,
        Some(Commands::Profile(args)) => {
            topk::commands::profile::run(config, args, cli.output, &mut std::io::stdout())
        }

        #[cfg(feature = "import")]
        Some(Commands::Import(args)) => topk::commands::import::run(config, args, cli.output).await,

        Some(Commands::Completions { shell }) => {
            let mut script = Vec::new();
            generate(*shell, &mut Cli::command(), "topk", &mut script);
            std::io::stdout().write_all(&script)?;
            Ok(ExitCode::SUCCESS)
        }

        None => {
            Cli::command().print_help()?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Warnings by default; `RUST_LOG` overrides verbosity. Stdout carries results.
fn init_logging(verbose: u8) {
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| match verbose {
            0 => tracing_subscriber::EnvFilter::new("warn"),
            1 => tracing_subscriber::EnvFilter::new("warn,topk=info,topk_rs=warn,topk_es=warn"),
            _ => tracing_subscriber::EnvFilter::new("warn,topk=debug,topk_rs=warn,topk_es=warn"),
        });
    let progress = IndicatifLayer::new()
        .with_progress_style(ProgressStyle::default_spinner())
        .with_tick_settings(TickSettings {
            term_draw_hz: 4,
            default_tick_interval: Some(std::time::Duration::from_millis(250)),
            ..Default::default()
        });
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(std::io::stderr().is_terminal())
                .with_writer(progress.get_stderr_writer())
                .with_filter(filter),
        )
        .with(progress.with_filter(IndicatifFilter::new(false)))
        .init();
}
