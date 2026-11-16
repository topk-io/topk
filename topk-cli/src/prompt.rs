use std::io::{stdin, ErrorKind, IsTerminal};

use anyhow::{bail, ensure, Result};
use dialoguer::{Confirm, Password, Select};

pub struct Prompt;

impl Prompt {
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
        let value = Password::new().with_prompt(prompt).interact()?;
        let value = value.trim();
        ensure!(!value.is_empty(), "nothing was entered");
        Ok(value.to_string())
    }

    pub fn confirm(prompt: String, yes: bool) -> Result<bool> {
        if yes {
            return Ok(true);
        }
        if !stdin().is_terminal() {
            bail!("stdin is not a terminal");
        }
        match Confirm::new().with_prompt(prompt).default(false).interact() {
            Ok(confirmed) => Ok(confirmed),
            Err(dialoguer::Error::IO(error)) if error.kind() == ErrorKind::Interrupted => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}
