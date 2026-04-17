use std::io::IsTerminal;

use anyhow::Result;
use dialoguer::{Password, Select};

use crate::config;
use crate::output::{GREEN, RESET};

/// Resolves the API key for the current invocation.
///
/// When `explicit` is `false` (any command that needs credentials):
///   1. Returns the provided key (CLI flag / env var) if present.
///   2. Returns the key from the config file if present.
///   3. On a TTY, prints "No API key found." and shows the interactive menu.
///   4. On a non-TTY, returns an error.
///
/// When `explicit` is `true` (`topk auth`):
///   Shows the current key status, then always presents the interactive menu
///   regardless of whether a key already exists.
///
/// Returns `None` only for explicit `topk auth` when the user chose Skip.
pub fn resolve(
    provided: Option<String>,
    host: &str,
    https: bool,
    explicit: bool,
) -> Result<Option<String>> {
    if !explicit {
        if let Some(key) = provided {
            return Ok(Some(key));
        }
        if let Some(key) = config::load().api_key {
            return Ok(Some(key));
        }
        if !std::io::stderr().is_terminal() {
            anyhow::bail!(
                "API key not set. Set TOPK_API_KEY environment variable or run: topk auth"
            );
        }
        eprintln!("No API key found.");
        return prompt_menu(host, https)?
            .ok_or_else(|| anyhow::anyhow!("API key not set. Run `topk auth` to configure one"))
            .map(Some);
    }

    // Explicit auth: show current status, then always prompt.
    if let Ok(key) = std::env::var("TOPK_API_KEY") {
        eprintln!(
            "{GREEN}✓{RESET} API key set in TOPK_API_KEY environment variable ({})",
            config::mask(&key)
        );
    } else {
        let cfg = config::load();
        if let Some(key) = &cfg.api_key {
            let path = config::config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            eprintln!(
                "{GREEN}✓{RESET} API key set in \"{}\" ({})",
                path,
                config::mask(key)
            );
        } else {
            eprintln!("No API key found.");
        }
    }
    eprintln!();
    prompt_menu(host, https)
}

/// Presents the three-option menu and returns the resolved key, or `None` if
/// the user chose Skip.
pub fn prompt_menu(host: &str, https: bool) -> Result<Option<String>> {
    let options = ["Get your API key", "I already have an API key", "Skip"];

    let choice = Select::new().items(&options).default(0).interact()?;

    match choice {
        0 => {
            let scheme = if https { "https" } else { "http" };
            let url = format!("{}://console.{}/api-key", scheme, host);
            eprintln!("Opening {} in your browser...", url);
            if open::that(&url).is_err() {
                eprintln!("(could not open browser — visit the URL above manually)");
            }
            Ok(Some(prompt_and_save()?))
        }
        1 => Ok(Some(prompt_and_save()?)),
        _ => {
            eprintln!("Skipped. Run `topk auth` when you're ready.");
            Ok(None)
        }
    }
}

/// Prompts for the API key (hidden input), saves it to the config file, and returns it.
pub fn prompt_and_save() -> Result<String> {
    let key = Password::new().with_prompt("API key").interact()?;
    let key = key.trim().to_string();

    if key.is_empty() {
        anyhow::bail!("no API key provided");
    }

    let mut cfg = config::load();
    cfg.api_key = Some(key.clone());
    config::save(&cfg)?;

    let path = config::config_path()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    eprintln!("API key saved to {}", path);

    Ok(key)
}
