use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use dialoguer::{console::Term, Confirm, Input};
use serde::Serialize;

use crate::util::Spinner;

pub const GREEN: &str = "\x1b[32m";
pub const RED: &str = "\x1b[31m";
pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const BLUE: &str = "\x1b[34m";

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

pub trait RenderForHuman: Serialize {
    fn render(&self) -> impl Into<String>;
}

#[derive(Clone, Copy)]
pub struct Output {
    format: OutputFormat,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn print<T: RenderForHuman>(&self, value: &T) -> Result<(), serde_json::Error> {
        match self.format {
            OutputFormat::Json => self.print_json(value),
            OutputFormat::Text => self.print_text(value),
        }
    }

    pub fn clear_progress(&self) {
        if std::io::stderr().is_terminal() {
            eprint!("\r\x1b[2K");
            let _ = std::io::stderr().flush();
        }
    }

    pub fn print_text<T: RenderForHuman>(&self, value: &T) -> Result<(), serde_json::Error> {
        self.clear_progress();
        let rendered: String = value.render().into();
        if !rendered.is_empty() {
            println!("{rendered}");
        }
        Ok(())
    }

    pub fn print_json<T: Serialize>(&self, value: &T) -> Result<(), serde_json::Error> {
        println!("{}", serde_json::to_string(value)?);
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
        if matches!(self.format, OutputFormat::Text) {
            eprintln!("{GREEN}✓{RESET} {msg}");
        }
    }

    pub fn warn(&self, msg: &str) {
        if matches!(self.format, OutputFormat::Text) {
            eprintln!("{msg}");
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    pub fn prompt_dir(&self, prompt: impl Into<String>) -> std::io::Result<Option<PathBuf>> {
        if self.is_json() {
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

    pub fn confirm(&self, prompt: &str) -> std::io::Result<bool> {
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
                .wait_for_newline(true)
                .interact_on(&term)
                .map_err(std::io::Error::other);
        }

        #[cfg(not(unix))]
        {
            if std::io::stdin().is_terminal() {
                return Confirm::new()
                    .with_prompt(prompt)
                    .default(false)
                    .wait_for_newline(true)
                    .interact()
                    .map_err(std::io::Error::other);
            }

            Ok(false)
        }
    }

    pub fn confirm_or_yes(&self, prompt: &str, yes: bool) -> std::io::Result<bool> {
        if yes {
            Ok(true)
        } else {
            self.confirm(prompt)
        }
    }

    pub fn error(&self, e: &anyhow::Error) {
        match self.format {
            OutputFormat::Json => {
                let payload = serde_json::json!({ "error": format!("{:#}", e) });
                eprintln!("{payload}");
            }
            OutputFormat::Text => eprintln!("{BOLD}{RED}error:{RESET} {:#}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use tempfile::tempdir;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test]
    fn missing_api_key_error() {
        let config_home = tempdir().unwrap();
        let home = tempdir().unwrap();

        let out = cmd()
            .env_remove("TOPK_API_KEY")
            .env("XDG_CONFIG_HOME", config_home.path())
            .env("HOME", home.path())
            .args(["-o", "json", "dataset", "list"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        let error = parsed["error"].as_str().unwrap();
        assert!(error.contains("API key not set"));
        assert!(error.contains("TOPK_API_KEY"));
    }

}
