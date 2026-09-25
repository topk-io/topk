use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::client::auth;
use crate::endpoint::ManagementEndpoint;
use crate::management::ProjectToken;

#[derive(Args, Debug)]
pub struct LogoutArgs {
    #[command(flatten)]
    pub mgmt: ManagementEndpoint,
}

pub async fn run(args: &LogoutArgs) -> Result<ExitCode> {
    let auth = auth(&args.mgmt)?;
    auth.logout().await?;
    ProjectToken::clear_all(auth.sessions())?;
    eprintln!("{} Logged out.", "✓".green());
    Ok(ExitCode::SUCCESS)
}
