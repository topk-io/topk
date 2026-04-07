use std::io::{IsTerminal, Write};

use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Table};
use serde::Serialize;

use crate::util::{confirm, Spinner};

const GREEN: &str = "\x1b[32m";
pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const BLUE: &str = "\x1b[34m";

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
pub enum OutputArg {
    #[default]
    Human,
    Agent,
}

#[derive(Debug, Clone, Copy)]
pub enum JsonFormat {
    Compact,
    Pretty,
}

#[derive(Debug, Clone, Copy)]
pub enum OutputMode {
    Human,
    Agent(JsonFormat),
}

pub trait RenderForHuman: Serialize {
    fn render(&self) -> String;
}

pub struct Output {
    mode: OutputMode,
}

impl Output {
    pub fn new(agent_flag: bool, output: OutputArg, pretty: bool) -> Self {
        let format = if pretty {
            JsonFormat::Pretty
        } else {
            JsonFormat::Compact
        };
        let mode =
            if agent_flag || matches!(output, OutputArg::Agent) || !std::io::stdout().is_terminal()
            {
                OutputMode::Agent(format)
            } else {
                OutputMode::Human
            };
        Self { mode }
    }

    pub fn print<T: RenderForHuman>(&self, value: &T) -> Result<(), serde_json::Error> {
        match self.mode {
            OutputMode::Agent(fmt) => println!("{}", self.serialize(fmt, value)?),
            OutputMode::Human => {
                clear_progress();
                println!("{}", value.render());
            }
        }
        Ok(())
    }

    pub fn spinner(&self, msg: impl Into<String>) -> Spinner {
        match self.mode {
            OutputMode::Human => Spinner::with_elapsed(msg),
            OutputMode::Agent(_) => Spinner::disabled(),
        }
    }

    pub fn progress(&self, msg: &str) {
        if matches!(self.mode, OutputMode::Human) {
            progress(msg);
        }
    }

    pub fn success(&self, msg: &str) {
        if matches!(self.mode, OutputMode::Human) {
            eprintln!("{GREEN}✓{RESET} {msg}");
        }
    }

    pub fn confirm(&self, prompt: &str) -> std::io::Result<bool> {
        if matches!(self.mode, OutputMode::Human)
            && std::io::stdin().is_terminal()
            && std::io::stderr().is_terminal()
        {
            confirm(prompt)
        } else {
            Ok(false)
        }
    }

    pub fn error(&self, e: &anyhow::Error) {
        let payload = serde_json::json!({ "error": format!("{:#}", e) });
        match self.mode {
            OutputMode::Agent(fmt) => eprintln!("{}", self.serialize(fmt, &payload).unwrap()),
            OutputMode::Human => eprintln!("Error: {:#}", e),
        }
    }

    fn serialize<T: Serialize>(
        &self,
        fmt: JsonFormat,
        value: &T,
    ) -> Result<String, serde_json::Error> {
        match fmt {
            JsonFormat::Pretty => serde_json::to_string_pretty(value),
            JsonFormat::Compact => serde_json::to_string(value),
        }
    }
}

/// Prints a temporary progress message to stderr, overwriting the previous one.
/// Always ephemeral — never appears in final stdout output.
pub fn progress(msg: &str) {
    eprint!("\r\x1b[2K{}", msg);
    let _ = std::io::stderr().flush();
}

fn clear_progress() {
    eprint!("\r\x1b[2K");
    let _ = std::io::stderr().flush();
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
            .args(["--json", "dataset", "list"])
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
            .args(["--json", "dataset", "list"])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("TOPK_REGION"));
    }
}
