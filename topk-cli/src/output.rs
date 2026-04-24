use std::fmt::Display;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use colored::Colorize;
use dialoguer::{console::Term, Confirm, Input};
use serde::Serialize;
use topk_rs::Error;

use crate::util::progress::Spinner;

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Clone, Copy)]
pub struct Output {
    pub format: OutputFormat,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn print<T: Serialize + Display>(&self, value: &T) -> Result<(), Error> {
        match self.format {
            OutputFormat::Text => {
                let rendered = value.to_string();
                if !rendered.is_empty() {
                    println!("{rendered}");
                }
                Ok(())
            }

            OutputFormat::Json => {
                println!(
                    "{}",
                    serde_json::to_string(value)
                        .map_err(|e| Error::MalformedResponse(e.to_string()))?
                );
                Ok(())
            }
        }
    }

    pub fn print_json<T: Serialize>(&self, value: &T) -> Result<(), Error> {
        println!(
            "{}",
            serde_json::to_string(value).map_err(|e| Error::MalformedResponse(e.to_string()))?
        );
        Ok(())
    }

    pub fn print_json_line<T: Serialize>(&self, value: &T) -> Result<(), serde_json::Error> {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        serde_json::to_writer(&mut lock, value)?;
        writeln!(&mut lock).map_err(serde_json::Error::io)?;
        Ok(())
    }

    pub fn spinner(&self, msg: impl Into<String>) -> Spinner {
        match self.format {
            OutputFormat::Text => Spinner::with_elapsed(msg),
            OutputFormat::Json => Spinner::disabled(),
        }
    }

    pub fn success(&self, msg: &str) {
        match self.format {
            OutputFormat::Text => {
                eprintln!("{} {msg}", "✓".green());
            }
            OutputFormat::Json => {}
        }
    }

    pub fn warn(&self, msg: &str) {
        match self.format {
            OutputFormat::Text => {
                eprintln!("{msg}");
            }
            OutputFormat::Json => {}
        }
    }

    pub fn error(&self, e: &Error) {
        match self.format {
            OutputFormat::Json => {
                let payload = serde_json::json!({ "error": format!("{:#}", e) });
                eprintln!("{payload}");
            }
            OutputFormat::Text => eprintln!("{} {:#}", "error:".red().bold(), e),
        }
    }

    pub fn confirm(&self, prompt: &str) -> Result<bool, Error> {
        if !matches!(self.format, OutputFormat::Text) || !std::io::stdout().is_terminal() {
            return Ok(false);
        }

        #[cfg(unix)]
        {
            let tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")?;
            let term = Term::read_write_pair(tty.try_clone()?, tty);

            return Ok(Confirm::new()
                .with_prompt(prompt)
                .default(false)
                .wait_for_newline(true)
                .interact_on(&term)
                .map_err(std::io::Error::other)
                .map_err(Error::IoError)?);
        }

        #[cfg(not(unix))]
        {
            if std::io::stdin().is_terminal() {
                return Ok(Confirm::new()
                    .with_prompt(prompt)
                    .default(false)
                    .wait_for_newline(true)
                    .interact()
                    .map_err(std::io::Error::other)
                    .map_err(Error::IoError)?);
            }

            Ok(false)
        }
    }

    pub fn confirm_or_yes(&self, prompt: &str, yes: bool) -> Result<bool, Error> {
        if yes {
            Ok(true)
        } else {
            self.confirm(prompt)
        }
    }

    pub fn prompt_dir(&self, prompt: impl Into<String>) -> std::io::Result<Option<PathBuf>> {
        if matches!(self.format, OutputFormat::Json) {
            return Ok(None);
        }

        let prompt = prompt.into();

        #[cfg(unix)]
        {
            let tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")?;
            let term = Term::read_write_pair(tty.try_clone()?, tty);
            let input: String = Input::new()
                .with_prompt(prompt)
                .allow_empty(true)
                .interact_on(&term)
                .map_err(std::io::Error::other)?;
            let trimmed = input.trim().to_string();
            return Ok(if trimmed.is_empty() {
                None
            } else {
                Some(PathBuf::from(trimmed))
            });
        }

        #[cfg(not(unix))]
        {
            if std::io::stdin().is_terminal() {
                let input: String = Input::new()
                    .with_prompt(prompt)
                    .allow_empty(true)
                    .interact()
                    .map_err(std::io::Error::other)?;
                let trimmed = input.trim().to_string();
                return Ok(if trimmed.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(trimmed))
                });
            }
            Ok(None)
        }
    }
}
