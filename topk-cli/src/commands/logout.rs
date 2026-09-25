use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::auth::Auth;
use crate::config::Config;
use crate::endpoint::ManagementEndpoint;

#[derive(Args, Debug)]
pub struct LogoutArgs {
    #[command(flatten)]
    pub mgmt: ManagementEndpoint,
}

pub async fn run(args: &LogoutArgs) -> Result<ExitCode> {
    Auth::new(Config::new(args.mgmt.oauth.clone(), Config::dir()?))?
        .logout()
        .await?;
    eprintln!("{} Logged out.", "✓".green());
    Ok(ExitCode::SUCCESS)
}
