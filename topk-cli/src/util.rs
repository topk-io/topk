use std::io::{ErrorKind, IsTerminal};

use anyhow::{bail, ensure, Result};
use chrono::DateTime;
use dialoguer::{Confirm, Select};

pub fn select(prompt: &str, items: &[String], current: Option<usize>) -> Result<Option<usize>> {
    ensure!(
        std::io::stdin().is_terminal(),
        "pass a selection argument when stdin is not a terminal"
    );
    ensure!(!items.is_empty(), "no choices available");
    match Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(current.unwrap_or(0))
        .interact_opt()
    {
        Ok(selected) => Ok(selected),
        Err(dialoguer::Error::IO(error)) if error.kind() == ErrorKind::Interrupted => Ok(None),
        Err(error) => Err(error.into()),
    }
}

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
