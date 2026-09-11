use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use colored::Colorize;

use crate::endpoint::Endpoint;

// Registered Auth0 loopback callback ports.
const AUTH_CALLBACK_PORTS: [u16; 3] = [38123, 38124, 38125];

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Print the login URL instead of opening a browser.
    #[arg(long)]
    pub no_browser: bool,

    #[arg(long = "auth-callback-ports", env = "TOPK_AUTH_CALLBACK_PORTS", value_delimiter = ',', default_values_t = AUTH_CALLBACK_PORTS, hide = true)]
    pub callback_ports: Vec<u16>,
}

pub async fn run(endpoint: &Endpoint, args: &LoginArgs) -> Result<ExitCode> {
    let auth = endpoint.auth()?;
    let login = auth.login(&args.callback_ports).await?;
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
