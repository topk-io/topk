use std::io::{stdin, ErrorKind, IsTerminal};

use anyhow::{bail, ensure, Result};
use chrono::DateTime;
use dialoguer::{Confirm, Password, Select};

pub fn select(
    prompt: &str,
    items: &[impl ToString],
    current: Option<usize>,
) -> Result<Option<usize>> {
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

pub fn password(prompt: &str) -> Result<String> {
    let value = if stdin().is_terminal() {
        Password::new().with_prompt(prompt).interact()?
    } else {
        eprint!("{prompt}: ");
        let mut line = String::new();
        stdin().read_line(&mut line)?;
        line
    };
    Ok(value.trim().to_string())
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
