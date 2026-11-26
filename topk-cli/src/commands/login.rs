use std::env;
use std::io::{stdin, IsTerminal};
use std::process::ExitCode;

use anyhow::{ensure, Result};
use clap::Args;
use colored::Colorize;
use tracing::info;

use crate::auth::{AccessTokenClaims, Auth};
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

impl LoginArgs {
    fn browser(&self) -> bool {
        if self.no_browser {
            return false;
        }
        let set = |name| env::var_os(name).is_some_and(|value| !value.is_empty());
        if set("SSH_CONNECTION") || set("SSH_TTY") {
            return false;
        }
        if !cfg!(any(target_os = "macos", windows)) && !set("DISPLAY") && !set("WAYLAND_DISPLAY") {
            return false;
        }
        true
    }
}

pub async fn run(config: Config, args: &LoginArgs) -> Result<ExitCode> {
    info!(?args, "login");
    let profile = config.profile().name().label();
    let auth = Auth::new(config)?;
    let claims = if !args.browser() {
        prompt_code_login(&auth).await?
    } else {
        let login = auth.browser_login(&args.callback_ports).await?;
        match open::that_detached(login.url().as_str()) {
            Ok(()) => {
                eprintln!(
                    "Opening your browser to log in. If it doesn't open, visit:\n\n{}\n",
                    login.url()
                );
                eprintln!("Waiting for login...");
                login.finish().await?
            }
            Err(error) => {
                eprintln!("{} Failed opening a web browser\n  {error}\n", "!".red());
                prompt_code_login(&auth).await?
            }
        }
    };
    eprintln!("{} Logged in as {}{profile}", "✓".green(), claims.account());
    Ok(ExitCode::SUCCESS)
}

async fn prompt_code_login(auth: &Auth) -> Result<AccessTokenClaims> {
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
    login.finish(code).await
}
