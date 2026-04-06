use std::io::{IsTerminal, Write};

use serde::Serialize;
use topk_rs::proto::v1::data::{value, Value};

use crate::util::Spinner;

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
        let format = if pretty { JsonFormat::Pretty } else { JsonFormat::Compact };
        let mode = if agent_flag
            || matches!(output, OutputArg::Agent)
            || !std::io::stdout().is_terminal()
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

    pub fn error(&self, e: &anyhow::Error) {
        let payload = serde_json::json!({ "error": format!("{:#}", e) });
        match self.mode {
            OutputMode::Agent(fmt) => eprintln!("{}", self.serialize(fmt, &payload).unwrap()),
            OutputMode::Human => eprintln!("Error: {:#}", e),
        }
    }

    fn serialize<T: Serialize>(&self, fmt: JsonFormat, value: &T) -> Result<String, serde_json::Error> {
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

const CYAN_BOLD: &str = "\x1b[1;36m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const COL_GAP: usize = 4;

/// Formats a table from headers and rows of string values.
/// Headers are rendered in cyan bold, data rows in bold, with no borders.
pub fn table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let col_count = headers.len();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let mut out = String::new();

    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push_str(&" ".repeat(COL_GAP));
        }
        out.push_str(&format!("{}{:<width$}{}", CYAN_BOLD, h, RESET, width = widths[i]));
    }
    out.push('\n');

    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                out.push_str(&" ".repeat(COL_GAP));
            }
            let width = widths.get(i).copied().unwrap_or(0);
            out.push_str(&format!("{}{:<width$}{}", BOLD, cell, RESET, width = width));
        }
        out.push('\n');
    }

    out.trim_end_matches('\n').to_string()
}

/// Formats a proto Value as a compact string for table display.
pub fn format_value(v: &Value) -> String {
    match &v.value {
        Some(value::Value::String(s)) => s.clone(),
        Some(value::Value::Bool(b)) => b.to_string(),
        Some(value::Value::U32(n)) => n.to_string(),
        Some(value::Value::U64(n)) => n.to_string(),
        Some(value::Value::I32(n)) => n.to_string(),
        Some(value::Value::I64(n)) => n.to_string(),
        Some(value::Value::F32(n)) => format!("{:.4}", n),
        Some(value::Value::F64(n)) => format!("{:.4}", n),
        Some(value::Value::Null(_)) => "null".into(),
        None => "".into(),
        _ => unreachable!("unsupported value type: {:?}", v.value),
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
            .args(["--json", "dataset", "list"])
            .output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value = serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("TOPK_API_KEY"));
    }

    #[test]
    fn missing_region_error() {
        let out = cmd()
            .env("TOPK_API_KEY", "test-key")
            .env_remove("TOPK_REGION")
            .args(["--json", "dataset", "list"])
            .output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value = serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        assert!(parsed["error"].as_str().unwrap().contains("TOPK_REGION"));
    }
}
