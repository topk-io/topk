use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;
use tracing::info;

use crate::auth::Auth;
use crate::config::Config;

#[derive(Args, Debug)]
pub struct LogoutArgs {}

pub async fn run(config: Config, args: &LogoutArgs) -> Result<ExitCode> {
    info!(?args, "logout");
    let profile = config.profile().name().label();
    Auth::new(config)?.logout().await?;
    eprintln!("{} Logged out{profile}.", "✓".green());
    Ok(ExitCode::SUCCESS)
}
