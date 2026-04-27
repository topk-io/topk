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

    pub fn print_json_line<T: Serialize>(&self, value: &T) -> Result<(), Error> {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        serde_json::to_writer(&mut lock, value).map_err(map_json_write_error)?;
        writeln!(&mut lock).map_err(Error::IoError)?;
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

            return Confirm::new()
                .with_prompt(prompt)
                .default(false)
                .wait_for_newline(false)
                .interact_on(&term)
                .map_err(|e| {
                    let dialoguer::Error::IO(ref io_err) = e;
                    if io_err.kind() == std::io::ErrorKind::Interrupted {
                        let _ = term.show_cursor();
                        std::process::exit(130);
                    }
                    Error::IoError(std::io::Error::other(e))
                });
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

fn map_json_write_error(err: serde_json::Error) -> Error {
    if let Some(io_kind) = err.io_error_kind() {
        return Error::IoError(std::io::Error::from(io_kind));
    }

    Error::MalformedResponse(err.to_string())
}
