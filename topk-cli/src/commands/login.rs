use anyhow::Result;
use dialoguer::{Password, Select};
use serde::{Deserialize, Serialize};
use topk_rs::Error;

use crate::config;
use crate::output::RenderForHuman;

#[derive(Serialize, Deserialize)]
pub struct LoginResult {
    pub api_key: Option<String>,
}

impl RenderForHuman for LoginResult {
    fn render(&self) -> impl Into<String> {
        if self.api_key.is_some() {
            "API key saved.".to_string()
        } else {
            "Login skipped.".to_string()
        }
    }
}

fn console_url(https: bool, host: &str) -> String {
    let scheme = if https { "https" } else { "http" };
    format!("{}://console.{}/api-key", scheme, host)
}

pub fn run(host: &str, https: bool) -> Result<LoginResult, Error> {
    prompt_api_key(host, https)
}

/// Resolves the API key for any command that needs credentials.
pub fn resolve(api_key: Option<String>, config: &config::Config) -> Result<Option<String>, Error> {
    if let Some(key) = api_key {
        return Ok(Some(key));
    }
    if let Some(key) = config.api_key.clone() {
        return Ok(Some(key));
    }

    Err(Error::Input(anyhow::anyhow!(
        "API key not set. Set TOPK_API_KEY environment variable or run: topk login"
    ))
    .into())
}

fn prompt_api_key(host: &str, https: bool) -> Result<LoginResult, Error> {
    let options = ["Create a new API key", "Use an existing API key", "Skip"];

    let choice = Select::new()
        .with_prompt("How would you like to authenticate with TopK?")
        .items(&options)
        .default(0)
        .interact();

    match choice {
        Ok(0) => {
            let url = console_url(https, host);
            let _ = open::that(&url);
            prompt_and_save()
        }
        Ok(1) => prompt_and_save(),
        Ok(_) => Ok(LoginResult { api_key: None }),
        Err(e) => Err(Error::Input(anyhow::anyhow!(e.to_string()))),
    }
}

fn prompt_and_save() -> Result<LoginResult, Error> {
    let password = Password::new().with_prompt("API key").interact();

    let api_key = match password {
        Ok(password) => password.trim().to_string(),
        Err(e) => return Err(Error::Input(anyhow::anyhow!(e.to_string()))),
    };

    if api_key.is_empty() {
        return Err(Error::Input(anyhow::anyhow!("no API key provided")));
    }

    Ok(LoginResult {
        api_key: Some(api_key),
    })
}
