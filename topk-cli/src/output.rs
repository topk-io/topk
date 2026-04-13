use std::io::{IsTerminal, Write};
use std::path::PathBuf;

use serde::Serialize;

use crate::util::{confirm, Spinner};

pub const GREEN: &str = "\x1b[32m";
pub const RED: &str = "\x1b[31m";
pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const BLUE: &str = "\x1b[34m";

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    #[value(alias = "text")]
    HumanReadable,
    Json,
}

pub trait RenderForHuman: Serialize {
    fn render(&self) -> impl Into<String>;
}

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
            OutputFormat::HumanReadable => self.print_human(value),
        }
    }

    pub fn print_human<T: RenderForHuman>(&self, value: &T) -> Result<(), serde_json::Error> {
        clear_progress();
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
            OutputFormat::HumanReadable => Spinner::with_elapsed(msg),
            OutputFormat::Json => Spinner::disabled(),
        }
    }

    pub fn progress(&self, msg: &str) {
        if matches!(self.format, OutputFormat::HumanReadable) {
            progress(msg);
        }
    }

    pub fn success(&self, msg: &str) {
        if matches!(self.format, OutputFormat::HumanReadable) {
            eprintln!("{GREEN}✓{RESET} {msg}");
        }
    }

    pub fn is_human(&self) -> bool {
        matches!(self.format, OutputFormat::HumanReadable)
    }

    pub fn confirm(&self, prompt: &str) -> std::io::Result<bool> {
        if matches!(self.format, OutputFormat::HumanReadable)
            && std::io::stdin().is_terminal()
            && std::io::stdout().is_terminal()
        {
            confirm(prompt)
        } else {
            Ok(false)
        }
    }

    pub fn prompt_dir(&self, prompt: &str) -> std::io::Result<Option<PathBuf>> {
        if !matches!(self.format, OutputFormat::HumanReadable)
            || !std::io::stdin().is_terminal()
        {
            return Ok(None);
        }
        eprint!("{}", prompt);
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(trimmed)))
        }
    }

    pub fn error(&self, e: &anyhow::Error) {
        match self.format {
            OutputFormat::Json => {
                let payload = serde_json::json!({ "error": format!("{:#}", e) });
                eprintln!(
                    "{}",
                    serde_json::to_string(&payload)
                        .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string())
                );
            }
            OutputFormat::HumanReadable => eprintln!("{BOLD}{RED}error:{RESET} {:#}", e),
        }
    }
}

/// Prints a temporary progress message to stderr, overwriting the previous one.
/// Always ephemeral — never appears in final stdout output.
pub fn progress(msg: &str) {
    if std::io::stderr().is_terminal() {
        eprint!("\r\x1b[2K{}", msg);
        let _ = std::io::stderr().flush();
    }
}

fn clear_progress() {
    if std::io::stderr().is_terminal() {
        eprint!("\r\x1b[2K");
        let _ = std::io::stderr().flush();
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test]
    fn missing_api_key_error() {
        let out = cmd()
            .env_remove("TOPK_API_KEY")
            .env_remove("TOPK_REGION")
            .args(["-o", "json", "dataset", "list"])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("TOPK_API_KEY"));
    }

    #[test]
    fn missing_region_error() {
        let out = cmd()
            .env("TOPK_API_KEY", "test-key")
            .env_remove("TOPK_REGION")
            .args(["-o", "json", "dataset", "list"])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("TOPK_REGION"));
    }

    #[test]
    fn completions_zsh() {
        let out = cmd().args(["completions", "zsh"]).output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("#compdef topk"));
    }

    #[test]
    fn completions_bash() {
        let out = cmd().args(["completions", "bash"]).output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("topk"));
    }
}
