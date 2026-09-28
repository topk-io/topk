use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::auth::Auth;
use crate::config::Config;

#[derive(Args, Debug)]
pub struct LogoutArgs {}

pub async fn run(config: Config, _: &LogoutArgs) -> Result<ExitCode> {
    Auth::new(config)?.logout().await?;
    eprintln!("{} Logged out.", "✓".green());
    Ok(ExitCode::SUCCESS)
}
