use std::collections::HashMap;
use std::io::{IsTerminal, Write};

use serde::Serialize;
use serde_json::json;
use tabled::{builder::Builder, settings::Style};
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

#[derive(Serialize)]
pub struct Aborted {
    aborted: bool,
}

pub const ABORTED: Aborted = Aborted { aborted: true };

impl RenderForHuman for Aborted {
    fn render(&self) -> String {
        "Aborted.".to_string()
    }
}

pub struct Output {
    pub mode: OutputMode,
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

/// Formats a table from headers and rows of string values.
pub fn table(headers: Vec<&str>, rows: Vec<Vec<String>>) -> String {
    let mut builder = Builder::new();
    builder.push_record(headers);
    for row in rows {
        builder.push_record(row);
    }
    builder.build().with(Style::sharp()).to_string()
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

/// Converts a proto Value map to a serde_json object.
pub fn metadata_to_json(metadata: &HashMap<String, Value>) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = metadata
        .iter()
        .map(|(k, v)| (k.clone(), value_to_json(v)))
        .collect();
    serde_json::Value::Object(map)
}

pub fn value_to_json(v: &Value) -> serde_json::Value {
    match &v.value {
        Some(value::Value::String(s)) => json!(s),
        Some(value::Value::Bool(b)) => json!(b),
        Some(value::Value::U32(n)) => json!(n),
        Some(value::Value::U64(n)) => json!(n),
        Some(value::Value::I32(n)) => json!(n),
        Some(value::Value::I64(n)) => json!(n),
        Some(value::Value::F32(n)) => json!(n),
        Some(value::Value::F64(n)) => json!(n),
        Some(value::Value::Null(_)) => serde_json::Value::Null,
        None => serde_json::Value::Null,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test]
    fn missing_api_key_error_message() {
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
    fn missing_region_error_message() {
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
