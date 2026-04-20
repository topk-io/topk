use anyhow::Result;
use dialoguer::{Password, Select};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::output::{Output, RenderForHuman};

#[derive(Serialize, Deserialize)]
pub struct LoginResult {
    pub saved: bool,
    pub path: Option<String>,
    pub opened_url: Option<String>,
}

struct LoginState {
    result: LoginResult,
    api_key: Option<String>,
}

impl RenderForHuman for LoginResult {
    fn render(&self) -> impl Into<String> {
        if self.saved {
            match &self.path {
                Some(path) => format!("API key saved to {}", path),
                None => "API key saved.".to_string(),
            }
        } else {
            "Login skipped.".to_string()
        }
    }
}

fn console_url(https: bool, host: &str) -> String {
    let scheme = if https { "https" } else { "http" };
    format!("{}://console.{}/api-key", scheme, host)
}

pub fn run(
    _api_key: Option<String>,
    config: &mut config::Config,
    host: &str,
    https: bool,
    _output: &Output,
) -> Result<LoginResult> {
    let login = prompt_menu(host, https)?;
    if let Some(api_key) = login.api_key {
        config.api_key = Some(api_key);
    }
    Ok(login.result)
}

/// Resolves the API key for any command that needs credentials.
pub fn resolve(
    api_key: Option<String>,
    config: &config::Config,
) -> Result<Option<String>> {
    if let Some(key) = api_key {
        return Ok(Some(key));
    }
    if let Some(key) = config.api_key.clone() {
        return Ok(Some(key));
    }

    anyhow::bail!("API key not set. Set TOPK_API_KEY environment variable or run: topk login");
}

fn prompt_menu(host: &str, https: bool) -> Result<LoginState> {
    let options = ["Create a new API key", "Use an existing API key", "Skip"];

    let choice = Select::new()
        .with_prompt("How would you like to authenticate with TopK?")
        .items(&options)
        .default(0)
        .interact()?;

    match choice {
        0 => {
            let url = console_url(https, host);
            let _ = open::that(&url);
            prompt_and_save(Some(url))
        }
        1 => prompt_and_save(None),
        _ => Ok(LoginState {
            result: LoginResult {
                saved: false,
                path: None,
                opened_url: None,
            },
            api_key: None,
        }),
    }
}

fn prompt_and_save(opened_url: Option<String>) -> Result<LoginState> {
    let key = Password::new().with_prompt("API key").interact()?;
    let key = key.trim().to_string();

    if key.is_empty() {
        anyhow::bail!("no API key provided");
    }

    Ok(LoginState {
        result: LoginResult {
            saved: true,
            path: config::config_path().map(|p| p.display().to_string()),
            opened_url,
        },
        api_key: Some(key),
    })
}
