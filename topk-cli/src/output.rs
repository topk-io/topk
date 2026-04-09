use std::io::{IsTerminal, Write};

use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Table};
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
    Text,
    Json,
}

pub trait RenderForHuman: Serialize {
    fn render(&self) -> String;
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
            OutputFormat::Json => println!("{}", serde_json::to_string(value)?),
            OutputFormat::Text => {
                clear_progress();
                println!("{}", value.render());
            }
        }
        Ok(())
    }

    pub fn spinner(&self, msg: impl Into<String>) -> Spinner {
        match self.format {
            OutputFormat::Text => Spinner::with_elapsed(msg),
            OutputFormat::Json => Spinner::disabled(),
        }
    }

    pub fn progress(&self, msg: &str) {
        if matches!(self.format, OutputFormat::Text) {
            progress(msg);
        }
    }

    pub fn success(&self, msg: &str) {
        if matches!(self.format, OutputFormat::Text) {
            eprintln!("{GREEN}✓{RESET} {msg}");
        }
    }

    pub fn is_human(&self) -> bool {
        matches!(self.format, OutputFormat::Text)
    }

    pub fn confirm(&self, prompt: &str) -> std::io::Result<bool> {
        if matches!(self.format, OutputFormat::Text)
            && std::io::stdin().is_terminal()
            && std::io::stderr().is_terminal()
        {
            confirm(prompt)
        } else {
            Ok(false)
        }
    }

    pub fn error(&self, e: &anyhow::Error) {
        match self.format {
            OutputFormat::Json => {
                let payload = serde_json::json!({ "error": format!("{:#}", e) });
                eprintln!("{}", serde_json::to_string(&payload).unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string()));
            }
            OutputFormat::Text => eprintln!("{BOLD}{RED}error:{RESET} {:#}", e),
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

/// Formats a table from headers and rows of string values.
/// Headers are rendered in cyan bold, no borders, wraps to terminal width.
pub fn table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(
        headers
            .iter()
            .map(|h| Cell::new(h).add_attribute(Attribute::Bold).fg(Color::Cyan)),
    );

    for row in rows {
        table.add_row(row);
    }

    table.to_string()
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
