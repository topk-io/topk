use std::io::{stdin, IsTerminal};
use std::process::ExitCode;

use anyhow::{ensure, Result};
use clap::Args;
use colored::Colorize;

use crate::auth::Auth;
use crate::config::Config;
use crate::prompt::Prompt;

// Registered Auth0 loopback callback ports.
const AUTH_CALLBACK_PORTS: [u16; 3] = [38123, 38124, 38125];

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Print the login URL and prompt for the login code.
    #[arg(long)]
    pub no_browser: bool,

    #[arg(long = "auth-callback-ports", env = "TOPK_AUTH_CALLBACK_PORTS", value_delimiter = ',', default_values_t = AUTH_CALLBACK_PORTS, hide = true)]
    pub callback_ports: Vec<u16>,
}

pub async fn run(config: Config, args: &LoginArgs) -> Result<ExitCode> {
    let auth = Auth::new(config)?;
    let claims = if args.no_browser {
        let login = auth.browserless_login()?;
        eprintln!(
            "Open this URL in any browser to log in:\n\n{}\n",
            login.url()
        );
        let prompt = "Paste the code shown in the browser";
        let code = if stdin().is_terminal() {
            Prompt::password(prompt)?
        } else {
            // Piped input
            eprint!("{prompt}: ");
            let mut code = String::new();
            stdin().read_line(&mut code)?;
            code.trim().to_string()
        };
        ensure!(!code.is_empty(), "no code entered");
        login.finish(code).await?
    } else {
        let login = auth.browser_login(&args.callback_ports).await?;
        eprintln!(
            "Opening your browser to log in. If it doesn't open, visit:\n\n{}\n",
            login.url()
        );
        let _ = open::that_detached(login.url().as_str());
        eprintln!("Waiting for login...");
        login.finish().await?
    };
    eprintln!("{} Logged in as {}", "✓".green(), claims.account());
    Ok(ExitCode::SUCCESS)
}
