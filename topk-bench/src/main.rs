mod commands;
mod config;
mod data;
mod providers;
mod s3;
mod telemetry;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct BenchArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug, Clone)]
enum Commands {
    /// Ingest data into the benchmark
    Ingest(commands::ingest::IngestArgs),
    /// Query data from the benchmark
    Query(commands::query::QueryArgs),
    /// Cleanup benchmark
    Cleanup(commands::cleanup::CleanupArgs),
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // Install telemetry (logs & metrics)
    telemetry::install()?;

    // Force colored output
    colored::control::set_override(true);

    // Parse arguments
    let args = BenchArgs::parse();

    // Run command
    match args.command {
        Commands::Ingest(args) => commands::ingest::run(args).await?,
        Commands::Query(args) => commands::query::run(args).await?,
        Commands::Cleanup(args) => commands::cleanup::run(args).await?,
    }

    Ok(())
}
