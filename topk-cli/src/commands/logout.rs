use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::endpoint::ManagementEndpoint;
use crate::management::ProjectTokens;

#[derive(Args, Debug)]
pub struct LogoutArgs {
    #[command(flatten)]
    pub mgmt: ManagementEndpoint,
}

pub async fn run(args: &LogoutArgs) -> Result<ExitCode> {
    let auth = args.mgmt.auth()?;
    auth.logout().await?;
    ProjectTokens::clear(&auth)?;
    eprintln!("{} Logged out.", "✓".green());
    Ok(ExitCode::SUCCESS)
}
