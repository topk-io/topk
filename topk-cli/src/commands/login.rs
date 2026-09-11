use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::endpoint::Endpoint;

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Print the login URL instead of opening a browser.
    #[arg(long)]
    pub no_browser: bool,
}

pub async fn run(endpoint: &Endpoint, args: &LoginArgs) -> Result<ExitCode> {
    let auth = endpoint.auth()?;
    let login = auth.login().await?;
    if args.no_browser {
        eprintln!(
            "Open this URL in your browser to log in:\n\n{}\n",
            login.url()
        );
    } else {
        eprintln!(
            "Opening your browser to log in. If it doesn't open, visit:\n\n{}\n",
            login.url()
        );
        let _ = open::that_detached(login.url().as_str());
    }
    eprintln!("Waiting for login...");
    match login.finish().await? {
        Some(claims) => eprintln!("{} Logged in as {}", "✓".green(), claims.account()),
        None => eprintln!("{} Logged in.", "✓".green()),
    }
    Ok(ExitCode::SUCCESS)
}
