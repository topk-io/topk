use std::io::{stdin, ErrorKind, IsTerminal};

use anyhow::{bail, ensure, Result};
use dialoguer::console::Term;
use dialoguer::{Confirm, Password, Select};

use crate::util::Interrupted;

/// Keeps Ctrl+C in a prompt from killing the process while the cursor is hidden.
struct Interruptible(#[cfg(unix)] libc::sighandler_t);

impl Interruptible {
    fn new() -> Self {
        Self(
            #[cfg(unix)]
            unsafe {
                libc::signal(libc::SIGINT, libc::SIG_IGN)
            },
        )
    }
}

impl Drop for Interruptible {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::signal(libc::SIGINT, self.0);
        }
        // dialoguer returns early on errors without showing the cursor again
        let _ = Term::stderr().show_cursor();
    }
}

pub struct Prompt;

impl Prompt {
    pub fn select(prompt: &str, items: &[impl ToString], current: Option<usize>) -> Result<usize> {
        ensure!(!items.is_empty(), "no choices available");
        let _interruptible = Interruptible::new();
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
        let _interruptible = Interruptible::new();
        match Confirm::new().with_prompt(prompt).default(false).interact() {
            Ok(confirmed) => Ok(confirmed),
            Err(dialoguer::Error::IO(error)) if error.kind() == ErrorKind::Interrupted => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}
