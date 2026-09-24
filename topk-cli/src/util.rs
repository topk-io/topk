use std::io::{ErrorKind, IsTerminal};

use anyhow::{bail, Result};
use chrono::DateTime;
use dialoguer::Confirm;

pub fn confirm(prompt: String, yes: bool) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        bail!("stdin is not a terminal");
    }
    match Confirm::new().with_prompt(prompt).default(false).interact() {
        Ok(confirmed) => Ok(confirmed),
        Err(dialoguer::Error::IO(error)) if error.kind() == ErrorKind::Interrupted => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub fn timestamp(seconds: i64) -> String {
    DateTime::from_timestamp(seconds, 0)
        .map(|date| date.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| seconds.to_string())
}

pub fn redact(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    match chars.len() {
        0..=8 => "******".to_string(),
        n => format!("******{}", chars[n - 4..].iter().collect::<String>()),
    }
}
