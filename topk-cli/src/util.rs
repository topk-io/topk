use std::io::{ErrorKind, IsTerminal};

use anyhow::{bail, ensure, Result};
use chrono::DateTime;
use dialoguer::{Confirm, Select};

#[derive(Debug, thiserror::Error)]
#[error("interrupted")]
pub struct Interrupted;

pub fn select(prompt: &str, items: &[impl ToString], current: Option<usize>) -> Result<usize> {
    ensure!(!items.is_empty(), "no choices available");
    match Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(current.unwrap_or(0))
        .interact_opt()
    {
        Ok(Some(selected)) => Ok(selected),
        Ok(None) => Err(Interrupted.into()),
        Err(dialoguer::Error::IO(error)) if error.kind() == ErrorKind::Interrupted => {
            Err(Interrupted.into())
        }
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
