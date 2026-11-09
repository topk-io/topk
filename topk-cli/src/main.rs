use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;

use topk::auth::OAuthConfig;
use topk::config::Config;
use topk::host::Host;
use topk::output::Output;

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
    host: Host,

    #[command(flatten)]
    oauth: OAuthConfig,
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

    /// Run SQL against your collections
    Sql(topk::commands::sql::SqlArgs),

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
    // Rust ignores SIGPIPE, so `topk … | head` panics on the closed pipe.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL)
    };
    // AWS SSO for duckdb: sets AWS_CONFIG_FILE, UB once runtime threads getenv.
    #[cfg(feature = "import")]
    if std::env::args().any(|a| a == "import") {
        topk::import::source::aws_process_profile();
    }
    async_main()
}

/// `--project-id` defaults to the project `topk project select` saved; without
/// one, `sql` requires it and `import` requires it unless `--dry-run`.
fn command() -> clap::Command {
    // Clap's `required` ignores defaults, so a selected project lifts it.
    let selected: Option<&'static str> = selected_project().map(|id| &*id.leak());
    let command = Cli::command().mut_subcommand("sql", |sql| {
        sql.mut_arg("project_id", |arg| match selected {
            Some(id) => arg.default_value(id).required(false),
            None => arg,
        })
    });
    #[cfg(feature = "import")]
    let command = command.mut_subcommand("import", |import| {
        import.mut_arg("project_id", |arg| match selected {
            Some(id) => arg.default_value(id),
            None => arg.required_unless_present("dry_run"),
        })
    });
    command
}

/// Read before the real parse: `--host` and the auth flags pick where it is saved.
/// Unreadable means none, so a broken file cannot fail unrelated commands.
fn selected_project() -> Option<String> {
    let matches = Cli::command().ignore_errors(true).try_get_matches().ok()?;
    let config = Config::new(
        Host::from_arg_matches(&matches).ok()?,
        OAuthConfig::from_arg_matches(&matches).ok()?,
        Config::dir().ok()?,
    );
    Some(config.defaults().ok()?.project_id?.to_string())
}

#[tokio::main]
async fn async_main() -> ExitCode {
    let cli = Cli::from_arg_matches(&command().get_matches()).unwrap_or_else(|e| e.exit());
    init_logging(cli.verbose);
    match run(&cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{} {e:#}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    let config = Config::new(cli.host.clone(), cli.oauth.clone(), Config::dir()?);
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

        Some(Commands::Login(args)) => topk::commands::login::run(config, args).await,
        Some(Commands::Logout(args)) => topk::commands::logout::run(config, args).await,

        #[cfg(feature = "import")]
        Some(Commands::Import(args)) => topk::commands::import::run(config, args, cli.output).await,

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
